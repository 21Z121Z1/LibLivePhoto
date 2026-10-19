package org.liblivephoto.android;

import android.app.Instrumentation;
import android.os.*;
import android.system.*;
import org.liblivephoto.*;
import java.io.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;

/** Actual installed APK/Binder/JNI checks. No mocked Android or permission bypass. */
public final class ServiceAcceptance extends Instrumentation {
    private int assertions;
    private void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
        assertions++;
    }
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            runChecks(); result.putString("stream", "ANDROID_SERVICE_ACCEPTANCE passed; assertions=" + assertions + "\n");
            result.putInt("assertions", assertions); finish(-1, result);
        } catch (Throwable error) {
            android.util.Log.e("LibLivePhotoAcceptance", "native service acceptance failed", error);
            result.putString("stream", "ANDROID_SERVICE_ACCEPTANCE FAILED: " + error + "\n"); finish(0, result);
        }
    }
    private byte[] asset(String name) throws IOException {
        try (InputStream in = getContext().getAssets().open(name); ByteArrayOutputStream bytes = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[65536]; int n; while ((n=in.read(buffer))!=-1) bytes.write(buffer,0,n); return bytes.toByteArray();
        }
    }
    private File file(String name, byte[] bytes, int prefix) throws IOException {
        File file = new File(getTargetContext().getCacheDir(), name);
        try (OutputStream out = new FileOutputStream(file)) { out.write(new byte[prefix]); out.write(bytes); out.write(new byte[4]); }
        return file;
    }
    private NativeResult invoke(MotionPhotoClient client, Request request, MotionPhotoClient.Source p, MotionPhotoClient.Source m) throws Exception {
        return client.execute(request,p,m,null).result().get(40,TimeUnit.SECONDS);
    }
    private void runChecks() throws Exception {
        byte[] still=asset("cover.jpg"), movie=asset("motion.mp4"), motion=asset("motion-photo.jpg");
        File stillFile=file("own-still.bin",still,0), movieFile=file("own-movie.bin",movie,0), motionFile=file("own-motion-range.bin",motion,13);
        try (ParcelFileDescriptor stillFD=ParcelFileDescriptor.open(stillFile,ParcelFileDescriptor.MODE_READ_ONLY);
             ParcelFileDescriptor movieFD=ParcelFileDescriptor.open(movieFile,ParcelFileDescriptor.MODE_READ_ONLY);
             ParcelFileDescriptor motionFD=ParcelFileDescriptor.open(motionFile,ParcelFileDescriptor.MODE_READ_ONLY);
             MotionPhotoClient parser=new MotionPhotoClient(getTargetContext(),MetadataParserService.class);
             MotionPhotoClient builder=new MotionPhotoClient(getTargetContext(),MotionPhotoBuilderService.class);
             MotionPhotoClient checker=new MotionPhotoClient(getTargetContext(),MotionPhotoCheckerService.class);
             MotionPhotoClient extractor=new MotionPhotoClient(getTargetContext(),MotionPhotoExtractorService.class)) {
            for (MotionPhotoClient client : Arrays.asList(parser,builder,checker,extractor)) {
                client.ready().get(20,TimeUnit.SECONDS);
                check(client.remoteProcessId()!=android.os.Process.myPid(),"service must actually use another process");
                check(invoke(client,new Request("version"),null,null).success,"remote Rust native library loads");
            }
            Os.lseek(motionFD.getFileDescriptor(),7,OsConstants.SEEK_SET);
            MotionPhotoClient.Source source=MotionPhotoClient.Source.range(motionFD,13,motion.length);
            AtomicInteger readProgress=new AtomicInteger();
            NativeResult metadata=parser.execute(new Request("metadata"),source,null,(p,n,t)->{if("read".equals(p)&&n>0)readProgress.incrementAndGet();}).result().get(40,TimeUnit.SECONDS);
            check(metadata.success,"isolated metadata result");
            check(readProgress.get()>0,"actual FD read progress crosses Binder");
            check(Os.lseek(motionFD.getFileDescriptor(),0,OsConstants.SEEK_CUR)==7,"caller file cursor unchanged");
            check(Os.fstat(motionFD.getFileDescriptor()).st_size==motionFile.length(),"caller descriptor remains open");
            NativeResult parsed=invoke(parser,new Request("parse"),source,null);
            check(parsed.success&&parsed.headerJson.contains("812500"),"exact photo time through isolated parser/JNI");
            check(invoke(checker,new Request("check"),source,null).success,"checker service");
            NativeResult extracted=invoke(extractor,new Request("extract").option("still",false).option("video",true).option("xmp",false),source,null);
            check(extracted.success&&extracted.files.size()==1&&Arrays.equals(movie,extracted.files.get(0).copyBytes()),"extractor returns byte-exact movie over pipe");
            MotionPhotoClient.Source stillSource=MotionPhotoClient.Source.range(stillFD,0,still.length), movieSource=MotionPhotoClient.Source.range(movieFD,0,movie.length);
            NativeResult built=invoke(builder,new Request("build").inputKind("parts").target("google-jpeg").time(new ExactTime(13,16)),stillSource,movieSource);
            check(built.success&&!built.files.isEmpty(),"builder service executes real Rust plan");
            check(!invoke(builder,new Request("build").inputKind("parts").target("google-jpeg").time(new ExactTime(1,3)),stillSource,movieSource).success,"inexact time explicitly rejected");
            try { invoke(checker,new Request("build"),source,null); throw new AssertionError("checker accepted build"); }
            catch (IllegalArgumentException expected) { assertions++; }
            try { MotionPhotoClient.Source.range(motionFD,Long.MAX_VALUE-1,4); throw new AssertionError("range overflow accepted"); }
            catch (IllegalArgumentException expected) { assertions++; }
            NativeResult outOfBounds=invoke(parser,new Request("metadata"),MotionPhotoClient.Source.range(motionFD,13,motion.length+100),null);
            check(!outOfBounds.success&&outOfBounds.files.isEmpty(),"range beyond source diagnosed without publication");
            for (int i=0;i<8;i++) check(invoke(parser,new Request("parse"),source,null).success,"repeated service response/descriptor lifetime");
            // A large FD read keeps cancellation observable without replacing
            // the core with a delay/mock service. No private media is used.
            File large=new File(getTargetContext().getCacheDir(),"own-cancel.bin");
            try (RandomAccessFile fill=new RandomAccessFile(large,"rw")) { fill.setLength(32L*1024*1024); }
            try (ParcelFileDescriptor fd=ParcelFileDescriptor.open(large,ParcelFileDescriptor.MODE_READ_ONLY)) {
                MotionPhotoClient.Operation pending=parser.execute(new Request("parse"),MotionPhotoClient.Source.whole(fd),null,null);
                check(pending.cancel(),"cancel an actual service operation");
                try { pending.result().get(5,TimeUnit.SECONDS); throw new AssertionError("cancel returned success"); }
                catch (CancellationException | ExecutionException expected) { assertions++; }
                check(Os.fstat(fd.getFileDescriptor()).st_size==32L*1024*1024,"cancel keeps caller-owned FD open");
            } finally { large.delete(); }
        } finally { stillFile.delete();movieFile.delete();motionFile.delete(); }
    }
}
