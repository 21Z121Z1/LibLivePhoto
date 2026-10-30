package org.liblivephoto.android;
public final class MotionPhotoCheckerService extends MotionPhotoService {
    protected boolean allows(String action) { return "version".equals(action) || "check".equals(action); }
}
