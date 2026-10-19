package org.liblivephoto.android;
import android.os.ParcelFileDescriptor;
import org.liblivephoto.android.IOperationCallback;
/** API v1. Regular input files and explicit ranges; output must be a pipe.
 * Descriptor copies received by the service are always closed there.
 * No paths or media payloads are carried in Binder transactions. */
interface IMotionPhotoService {
    int apiVersion();
    int processId();
    long submit(String requestJson,
                in @nullable ParcelFileDescriptor primary, long primaryOffset, long primaryLength,
                in @nullable ParcelFileDescriptor movie, long movieOffset, long movieLength,
                in ParcelFileDescriptor outputPipe, IOperationCallback callback);
    void cancel(long operationId);
}
