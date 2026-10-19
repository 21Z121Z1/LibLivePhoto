package org.liblivephoto.android;
public final class MotionPhotoExtractorService extends MotionPhotoService {
    protected boolean allows(String action) { return "version".equals(action) || "extract".equals(action) || "extract-metadata".equals(action) || "agtm".equals(action); }
}
