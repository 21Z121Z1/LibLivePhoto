package org.liblivephoto.android;
/** Isolated metadata parsing/classification, XMP, and existing AGTM extraction. */
public final class MetadataParserService extends MotionPhotoService {
    protected boolean allows(String action) {
        return "version".equals(action) || "parse".equals(action) || "metadata".equals(action)
                || "extract-metadata".equals(action) || "classify-metadata".equals(action) || "xmp".equals(action) || "agtm".equals(action);
    }
}
