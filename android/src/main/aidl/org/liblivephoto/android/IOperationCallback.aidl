package org.liblivephoto.android;
/** Only small progress/status messages cross Binder; media travels over FDs. */
oneway interface IOperationCallback {
    void progress(long operationId, String phase, long completed, long total);
    void finished(long operationId, boolean success, String diagnostic);
}
