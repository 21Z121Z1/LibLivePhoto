package org.liblivephoto.android;
/** Build/plan only. Moments construction is deliberately outside the contract. */
public final class MotionPhotoBuilderService extends MotionPhotoService {
    protected boolean allows(String action) { return "version".equals(action) || "build".equals(action); }
}
