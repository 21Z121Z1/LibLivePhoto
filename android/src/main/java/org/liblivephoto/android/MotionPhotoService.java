package org.liblivephoto.android;

import android.app.Service;
import android.content.Intent;
import android.os.*;
import android.system.Os;
import android.system.OsConstants;
import org.json.JSONObject;
import org.liblivephoto.*;
import java.io.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;

/** Same core as CLI/JNI, with bounded asynchronous FD IPC. No storage permission. */
public abstract class MotionPhotoService extends Service {
    public static final int MAX_INPUT_BYTES = 32 * 1024 * 1024;
    private final AtomicLong nextId = new AtomicLong(1);
    private final ConcurrentHashMap<Long, Work> active = new ConcurrentHashMap<>();
    private final ThreadPoolExecutor executor = new ThreadPoolExecutor(1, 1, 0, TimeUnit.SECONDS,
            new ArrayBlockingQueue<Runnable>(2));
    protected abstract boolean allows(String action);

    private void requireOwner() {
        // An isolated service has its own UID. Compare the Binder caller with
        // the installed application's UID, not Process.myUid().
        if (Binder.getCallingUid() != getApplicationInfo().uid)
            throw new SecurityException("only the owning application may use this service");
    }
    private static void close(ParcelFileDescriptor fd) { if (fd != null) try { fd.close(); } catch (IOException ignored) {} }
    private static ParcelFileDescriptor own(ParcelFileDescriptor fd) throws IOException {
        // In-process Binder calls do not marshal descriptors. Keep caller ownership.
        return fd == null ? null : Binder.getCallingPid() == android.os.Process.myPid()
                ? ParcelFileDescriptor.dup(fd.getFileDescriptor()) : fd;
    }
    private static void range(ParcelFileDescriptor fd, long offset, long length) {
        if (fd == null) { if (offset != 0 || length != 0) throw new IllegalArgumentException("range without input"); }
        else if (offset < 0 || length <= 0 || length > MAX_INPUT_BYTES || offset > Long.MAX_VALUE - length)
            throw new IllegalArgumentException("invalid input range/budget");
    }

    private final IMotionPhotoService.Stub binder = new IMotionPhotoService.Stub() {
        public int apiVersion() { requireOwner(); return 1; }
        public int processId() { requireOwner(); return android.os.Process.myPid(); }
        public long submit(String request, ParcelFileDescriptor primary, long po, long pl,
                           ParcelFileDescriptor movie, long mo, long ml, ParcelFileDescriptor output,
                           IOperationCallback callback) {
            ParcelFileDescriptor p = null, m = null, out = null;
            try {
                requireOwner(); p = own(primary); m = own(movie); out = own(output);
                if (request == null || request.length() > 65536 || callback == null || out == null)
                    throw new IllegalArgumentException("request, callback and pipe are required (64 KiB request limit)");
                String action = new JSONObject(request).getString("action");
                if (!allows(action)) throw new IllegalArgumentException("operation is unavailable in this service: " + action);
                range(p, po, pl); range(m, mo, ml);
                if (pl + ml > MAX_INPUT_BYTES) throw new IllegalArgumentException("aggregate service input limit is 32 MiB");
                if (!OsConstants.S_ISFIFO(Os.fstat(out.getFileDescriptor()).st_mode))
                    throw new IllegalArgumentException("output must be a pipe; file publication belongs to the client");
                Work work = new Work(nextId.getAndIncrement(), request, p, po, pl, m, mo, ml, out, callback);
                active.put(work.id, work);
                try { callback.asBinder().linkToDeath(work, 0); executor.execute(work); }
                catch (Exception e) { work.cancel("service queue full or callback died"); throw e; }
                return work.id;
            } catch (Exception e) {
                close(p); close(m); close(out);
                throw new IllegalArgumentException(e.getMessage(), e);
            } finally {
                // If validation failed before taking ownership, close the IPC
                // descriptor copies too. The same-process originals stay open.
                if (Binder.getCallingPid() != android.os.Process.myPid()) {
                    if (p == null) close(primary); if (m == null) close(movie); if (out == null) close(output);
                }
            }
        }
        public void cancel(long id) { requireOwner(); Work work = active.get(id); if (work != null) work.cancel("cancelled"); }
    };

    private final class Work implements Runnable, IBinder.DeathRecipient {
        final long id, po, pl, mo, ml;
        final String request;
        final ParcelFileDescriptor primary, movie, output;
        final IOperationCallback callback;
        final AtomicBoolean terminal = new AtomicBoolean(), cleaned = new AtomicBoolean();
        Work(long id, String request, ParcelFileDescriptor p, long po, long pl,
             ParcelFileDescriptor m, long mo, long ml, ParcelFileDescriptor out, IOperationCallback callback) {
            this.id=id;this.request=request;primary=p;this.po=po;this.pl=pl;movie=m;this.mo=mo;this.ml=ml;output=out;this.callback=callback;
        }
        public void binderDied() { cancel("client process died"); }
        void cleanup() {
            if (!cleaned.compareAndSet(false, true)) return;
            active.remove(id); try { callback.asBinder().unlinkToDeath(this, 0); } catch (NoSuchElementException ignored) {}
            close(primary); close(movie); close(output);
        }
        void cancel(String reason) {
            if (!terminal.compareAndSet(false, true)) return;
            executor.remove(this);
            try { output.closeWithError(reason); } catch (IOException ignored) {}
            cleanup();
            try { callback.finished(id, false, reason); } catch (RemoteException ignored) {}
        }
        public void run() {
            if (terminal.get()) return;
            try {
                NativeEngine.Progress progress = (phase, n, total) -> {
                    if (terminal.get()) throw new CancellationException("cancelled");
                    try { callback.progress(id, phase, n, total); }
                    catch (RemoteException e) { throw new CancellationException("callback disconnected"); }
                };
                // AutoCloseInputStream owns only the service's IPC copies.
                try (FileInputStream p = primary == null ? null : new ParcelFileDescriptor.AutoCloseInputStream(primary);
                     FileInputStream m = movie == null ? null : new ParcelFileDescriptor.AutoCloseInputStream(movie)) {
                    NativeResult result;
                    try {
                        result = new NativeEngine().executeJson(request,
                                p == null ? null : Input.range(p.getChannel(), po, pl),
                                m == null ? null : Input.range(m.getChannel(), mo, ml), progress);
                    } catch (Exception | LinkageError e) {
                        if (terminal.get()) return;
                        result = NativeResult.failure(e.getClass().getSimpleName() + ": " + e.getMessage());
                    }
                    if (terminal.get()) return;
                    // A pipe can contain a partial response when cancelled or
                    // disconnected. Clients decode completely before publication.
                    try (OutputStream out = new ParcelFileDescriptor.AutoCloseOutputStream(output)) { result.writeEnvelope(out); }
                    if (terminal.compareAndSet(false, true)) {
                        callback.finished(id, result.success, result.success ? "complete" : "core returned diagnostics");
                    }
                }
            } catch (Throwable e) {
                cancel(e.getClass().getSimpleName() + ": " + e.getMessage());
            } finally { if (terminal.get()) cleanup(); }
        }
    }
    @Override public IBinder onBind(Intent intent) { return binder; }
    @Override public void onDestroy() {
        for (Work work : active.values()) work.cancel("service destroyed");
        executor.shutdownNow(); super.onDestroy();
    }
}
