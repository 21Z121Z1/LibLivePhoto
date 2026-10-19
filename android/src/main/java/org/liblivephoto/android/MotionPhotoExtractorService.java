package org.liblivephoto.android;
public final class MotionPhotoExtractorService extends MotionPhotoService {
    protected boolean allows(String action) { return "version".equals(action) || "extract".equals(action) || "agtm".equals(action); }
}
