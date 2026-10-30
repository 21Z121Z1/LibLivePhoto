package org.liblivephoto.android;

import android.content.*;
import android.os.*;
import org.liblivephoto.*;
import java.io.*;
import java.util.Set;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;

/** Explicit bound-service client. No uploads, file publication or permissions.
 * Results own their bytes. Keep supplied inputs open until execute() returns;
 * the client and service use independent descriptor copies thereafter. */
public final class MotionPhotoClient implements AutoCloseable {
    /** Package-scoped observer for acceptance diagnostics; never receives media. */
    interface BindingObserver { void event(String value); }
    private static final AtomicLong NEXT_CONNECTION = new AtomicLong();
    private final long connectionId = NEXT_CONNECTION.incrementAndGet();
    private final long bindingStarted = SystemClock.elapsedRealtime();
    private final BindingObserver bindingObserver;
    private final String serviceName;
    private final Context context;
    private final CompletableFuture<MotionPhotoClient> ready = new CompletableFuture<>();
    private final Set<Operation> active = ConcurrentHashMap.newKeySet();
    private final ThreadPoolExecutor readers = new ThreadPoolExecutor(2, 2, 0, TimeUnit.SECONDS,
            new ArrayBlockingQueue<Runnable>(8));
    private final AtomicBoolean closed = new AtomicBoolean();
    private volatile IMotionPhotoService service;
    private volatile boolean bound;

    private void trace(String event, String detail) {
        if (bindingObserver == null) return;
        String value = "connection=" + connectionId + " service=" + serviceName +
                " elapsedMs=" + (SystemClock.elapsedRealtime() - bindingStarted) +
                " pid=" + android.os.Process.myPid() + " uid=" + android.os.Process.myUid() +
                " tid=" + android.os.Process.myTid() +
                " main=" + (Looper.myLooper() == Looper.getMainLooper()) +
                " event=" + event + " " + detail;
        try { bindingObserver.event(value); } catch (Throwable ignored) {}
    }

    private final ServiceConnection connection = new ServiceConnection() {
        public void onServiceConnected(ComponentName name, IBinder binder) {
            trace("callback.connected", "closed=" + closed.get() + " binderAlive=" + binder.isBinderAlive());
            if (closed.get()) return;
            service = IMotionPhotoService.Stub.asInterface(binder);
            try {
                trace("handshake.begin", "");
                int version = service.apiVersion();
                trace("handshake.return", "version=" + version);
                if (version != 1) throw new IOException("service API mismatch");
                boolean completed = ready.complete(MotionPhotoClient.this);
                trace("ready.complete", "won=" + completed);
            } catch (Throwable e) { trace("handshake.error", e.getClass().getName()); fail(e); }
        }
        public void onServiceDisconnected(ComponentName name) { trace("callback.disconnected", ""); fail(new IOException("service process disconnected")); }
        @Override public void onBindingDied(ComponentName name) { trace("callback.bindingDied", ""); fail(new IOException("service binding died")); }
        @Override public void onNullBinding(ComponentName name) { trace("callback.nullBinding", ""); fail(new IOException("service returned no binder")); }
    };
    public MotionPhotoClient(Context context, Class<? extends MotionPhotoService> serviceClass) {
        this(context, serviceClass, null);
    }
    MotionPhotoClient(Context context, Class<? extends MotionPhotoService> serviceClass, BindingObserver observer) {
        bindingObserver = observer;
        serviceName = serviceClass.getSimpleName();
        this.context = context.getApplicationContext();
        try {
            trace("bind.begin", "");
            bound = this.context.bindService(new Intent(this.context, serviceClass), connection, Context.BIND_AUTO_CREATE);
            trace("bind.return", "accepted=" + bound);
            if (!bound) fail(new IOException("bindService returned false"));
        } catch (RuntimeException e) { trace("bind.error", e.getClass().getName()); fail(e); }
    }
    public CompletableFuture<MotionPhotoClient> ready() { return ready; }
    public int remoteProcessId() throws RemoteException {
        IMotionPhotoService remote = service;
        if (remote == null || closed.get()) throw new IllegalStateException("client is not connected");
        return remote.processId();
    }

    public static final class Source {
        final ParcelFileDescriptor descriptor; final long offset, length;
        private Source(ParcelFileDescriptor descriptor, long offset, long length) { this.descriptor=descriptor;this.offset=offset;this.length=length; }
        /** Borrowed descriptor. Cursor and lifetime remain the caller's. */
        public static Source range(ParcelFileDescriptor descriptor, long offset, long length) {
            if (descriptor == null || offset < 0 || length <= 0 || length > MotionPhotoService.MAX_INPUT_BYTES || offset > Long.MAX_VALUE-length)
                throw new IllegalArgumentException("invalid source range");
            return new Source(descriptor, offset, length);
        }
        public static Source whole(ParcelFileDescriptor descriptor) {
            return range(descriptor, 0, descriptor.getStatSize());
        }
    }
    /** Submit only after ready(). Media never travels through Binder byte arrays.
     * Completion and progress may arrive off the main thread. The caller owns UI dispatch. */
    public Operation execute(Request request, Source primary, Source movie, NativeEngine.Progress progress) throws IOException, RemoteException {
        IMotionPhotoService remote = service;
        if (remote == null || closed.get()) throw new IllegalStateException("client is not connected");
        if ((primary == null ? 0 : primary.length) + (movie == null ? 0 : movie.length) > MotionPhotoService.MAX_INPUT_BYTES)
            throw new IOException("aggregate service input limit is 32 MiB");
        ParcelFileDescriptor[] pipe = ParcelFileDescriptor.createReliablePipe();
        Operation op = new Operation(remote, pipe[0], progress); active.add(op);
        try (ParcelFileDescriptor p = primary == null ? null : ParcelFileDescriptor.dup(primary.descriptor.getFileDescriptor());
             ParcelFileDescriptor m = movie == null ? null : ParcelFileDescriptor.dup(movie.descriptor.getFileDescriptor());
             ParcelFileDescriptor out = pipe[1]) {
            readers.execute(op::read);
            long id = remote.submit(request.toJson(), p, primary == null ? 0 : primary.offset, primary == null ? 0 : primary.length,
                    m, movie == null ? 0 : movie.offset, movie == null ? 0 : movie.length, out, op.callback);
            op.id.set(id);
            if (op.terminal.get()) remote.cancel(id);
            return op;
        } catch (IOException | RemoteException | RuntimeException | Error e) {
            try { pipe[1].close(); } catch (IOException ignored) {}
            op.fail(e); throw e;
        }
    }
    public final class Operation {
        private final IMotionPhotoService remote;
        private final ParcelFileDescriptor input;
        private final NativeEngine.Progress progress;
        private final AtomicLong id = new AtomicLong();
        private final AtomicBoolean terminal = new AtomicBoolean();
        private final CompletableFuture<NativeResult> result = new CompletableFuture<>();
        private final IOperationCallback callback = new IOperationCallback.Stub() {
            public void progress(long operationId, String phase, long n, long total) {
                if (!terminal.get() && progress != null) try { progress.update(phase, n, total); } catch (RuntimeException ignored) {}
            }
            public void finished(long operationId, boolean success, String diagnostic) {
                // The complete envelope is authoritative. A cancellation/error
                // with a broken pipe is diagnosed by its reliable pipe reader.
            }
        };
        private Operation(IMotionPhotoService remote, ParcelFileDescriptor input, NativeEngine.Progress progress) { this.remote=remote;this.input=input;this.progress=progress; }
        public CompletableFuture<NativeResult> result() { return result; }
        private void read() {
            try (InputStream in = new ParcelFileDescriptor.AutoCloseInputStream(input)) {
                NativeResult value = NativeResult.readEnvelope(in, 160 * 1024 * 1024);
                if (terminal.compareAndSet(false, true)) result.complete(value);
            } catch (Throwable e) { fail(e); }
            finally { active.remove(this); }
        }
        private boolean fail(Throwable error) {
            if (terminal.compareAndSet(false, true)) {
                result.completeExceptionally(error);
                try { input.close(); } catch (IOException ignored) {}
                active.remove(this);
                long currentId = id.get(); if (currentId != 0) try { remote.cancel(currentId); } catch (RemoteException | RuntimeException ignored) {}
                return true;
            }
            return false;
        }
        public boolean cancel() {
            return fail(new CancellationException("cancelled"));
        }
    }
    private void fail(Throwable error) {
        ready.completeExceptionally(error); service = null;
        for (Operation op : active) op.fail(error);
    }
    @Override public void close() {
        if (!closed.compareAndSet(false, true)) return;
        trace("close.begin", "bound=" + bound);
        fail(new CancellationException("client closed"));
        if (bound) { trace("unbind.begin", ""); context.unbindService(connection); bound = false; trace("unbind.return", ""); }
        readers.shutdownNow();
        trace("close.return", "");
    }
}
