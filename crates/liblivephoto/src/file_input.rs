//! Bounded file snapshots. Unix positional reads leave the cursor unchanged.
//! Windows restores it after each read; shared handles need caller synchronization.
use std::fs::File;
use std::io;

use crate::{MediaTime, MotionPhoto, MotionPhotoReadError as Error, ParseOptions};

/// An exact, nonempty range in an open regular file. Offsets in the parsed model
/// are relative to this snapshot, not to the enclosing file. Zero is not a
/// sentinel for "the rest of the file".
/// On Windows the caller must synchronize other operations on this file or
/// cloned handles while a snapshot is being read (Rust `seek_read` moves the
/// shared cursor). The cursor is restored on return, including read errors.
#[derive(Debug, Clone, Copy)]
pub struct FileRange<'a> {
    pub file: &'a File,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum FileInput<'a> {
    SingleFile(FileRange<'a>),
    ApplePair {
        still: FileRange<'a>,
        movie: FileRange<'a>,
    },
    Parts {
        still: FileRange<'a>,
        movie: FileRange<'a>,
        presentation: Option<MediaTime>,
    },
}

pub(crate) enum OwnedInputKind {
    Single,
    ApplePair,
    Parts(Option<MediaTime>),
}

fn read_at(file: &File, bytes: &mut [u8], offset: u64) -> io::Result<usize> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_at(bytes, offset)
    }
    #[cfg(windows)]
    {
        use std::io::{Seek, SeekFrom};
        use std::os::windows::fs::FileExt;
        let mut borrowed = file;
        let position = borrowed.stream_position()?;
        let read = file.seek_read(bytes, offset);
        borrowed.seek(SeekFrom::Start(position))?;
        read
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (file, bytes, offset);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "positional file input is unavailable",
        ))
    }
}

fn snapshot(range: FileRange<'_>, budget: usize) -> Result<Vec<u8>, Error> {
    let end = range
        .offset
        .checked_add(range.length)
        .ok_or(Error::InvalidInputRange)?;
    if range.length == 0 || !range.file.metadata()?.is_file() || end > range.file.metadata()?.len()
    {
        return Err(Error::InvalidInputRange);
    }
    let length = usize::try_from(range.length).map_err(|_| Error::InputLimit)?;
    if length > budget {
        return Err(Error::InputLimit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| Error::InputLimit)?;
    bytes.resize(length, 0);
    let mut read = 0;
    while read < length {
        match read_at(range.file, &mut bytes[read..], range.offset + read as u64) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "input file changed during snapshot",
                )
                .into())
            }
            Ok(n) => read += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(bytes)
}

impl MotionPhoto<'static> {
    /// Read an owned snapshot from exact file ranges. The combined byte budget
    /// also applies to associations. Files and their cursors remain caller-owned.
    pub fn open_file(input: FileInput<'_>, options: ParseOptions) -> Result<Self, Error> {
        let (sources, kind) = match input {
            FileInput::SingleFile(range) => (
                vec![snapshot(range, options.max_input_bytes)?],
                OwnedInputKind::Single,
            ),
            FileInput::ApplePair { still, movie } | FileInput::Parts { still, movie, .. } => {
                let combined = still
                    .length
                    .checked_add(movie.length)
                    .ok_or(Error::InputLimit)?;
                if combined > options.max_input_bytes as u64 {
                    return Err(Error::InputLimit);
                }
                let still = snapshot(still, options.max_input_bytes)?;
                let movie = snapshot(movie, options.max_input_bytes - still.len())?;
                let kind = match input {
                    FileInput::Parts { presentation, .. } => OwnedInputKind::Parts(presentation),
                    _ => OwnedInputKind::ApplePair,
                };
                (vec![still, movie], kind)
            }
        };
        Self::from_owned_sources(sources, kind, options)
    }

    /// POSIX descriptor convenience. Duplicating a borrowed descriptor never
    /// transfers ownership and positional reads do not change its seek position.
    #[cfg(unix)]
    pub fn open_fd(
        fd: std::os::fd::BorrowedFd<'_>,
        offset: u64,
        length: u64,
        options: ParseOptions,
    ) -> Result<Self, Error> {
        let file = File::from(fd.try_clone_to_owned()?);
        Self::open_file(
            FileInput::SingleFile(FileRange {
                file: &file,
                offset,
                length,
            }),
            options,
        )
    }
}
