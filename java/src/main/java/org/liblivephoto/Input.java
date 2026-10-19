package org.liblivephoto;
import java.io.*;
import java.nio.ByteBuffer;
import java.nio.channels.FileChannel;
import java.nio.file.*;
/** Input snapshots. A supplied channel/descriptor remains caller-owned and open. */
public interface Input {
  byte[] snapshot(int budget,NativeEngine.Progress progress) throws IOException;
  static Input bytes(byte[] source){final byte[] retained=source.clone();return (budget,progress)->{if(retained.length>budget)throw new IOException("input budget exceeded");progress.update("read",retained.length,retained.length);return retained.clone();};}
  static Input path(Path path){return (budget,progress)->{try(FileChannel channel=FileChannel.open(path,StandardOpenOption.READ)){return read(channel,0,channel.size(),budget,progress,false);}};}
  /** Actual file-descriptor plus range capability via its public FileChannel.
   * Positional reads leave the cursor unchanged; this method never closes it. */
  static Input range(FileChannel channel,long offset,long length){if(offset<0||length<=0||offset>Long.MAX_VALUE-length)throw new IllegalArgumentException("invalid file range");return (budget,progress)->read(channel,offset,length,budget,progress,true);}
  static byte[] read(FileChannel channel,long offset,long length,int budget,NativeEngine.Progress progress,boolean ranged)throws IOException {
    if(length<0||length>budget||offset>Long.MAX_VALUE-length||offset+length>channel.size()||(ranged&&length==0))throw new IOException("input range/budget exceeded");
    byte[] data=new byte[(int)length];int done=0;progress.update("read",0,length);
    while(done<data.length){if(Thread.currentThread().isInterrupted())throw new InterruptedIOException("read cancelled");ByteBuffer buffer=ByteBuffer.wrap(data,done,Math.min(65536,data.length-done));int n=channel.read(buffer,offset+done);if(n<0)throw new EOFException("input changed during snapshot");if(n==0)throw new IOException("file read made no progress");done+=n;progress.update("read",done,length);}return data;
  }
}
