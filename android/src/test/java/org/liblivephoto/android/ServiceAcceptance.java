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
    private String phase = "instrumentation startup";
    private String scope = "all";
    private void markPhase(String value) {
        phase = value;
        android.util.Log.i("LibLivePhotoAcceptance", phase);
        Bundle progress = new Bundle();
        progress.putString("phase", phase);
        progress.putString("scope", scope);
        progress.putInt("assertions", assertions);
        sendStatus(1, progress);
    }
    private void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
        assertions++;
    }
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); if(arguments!=null)scope=arguments.getString("scope","all"); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            runChecks(); result.putString("stream", "ANDROID_SERVICE_ACCEPTANCE passed; scope="+scope+"; assertions=" + assertions + "\n");
            result.putInt("assertions", assertions); finish(-1, result);
        } catch (Throwable error) {
            android.util.Log.e("LibLivePhotoAcceptance", "native service acceptance failed", error);
            result.putString("phase",phase);
            result.putString("stack",android.util.Log.getStackTraceString(error));
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
        markPhase("operation "+request.toJson());
        return client.execute(request,p,m,null).result().get(40,TimeUnit.SECONDS);
    }
    private boolean scenario(String name) { return "all".equals(scope)||name.equals(scope); }
    private final class BindingTrace implements MotionPhotoClient.BindingObserver {
        private final ArrayDeque<String> events = new ArrayDeque<>();
        private int omitted;
        public synchronized void event(String value) {
            if (events.size() == 64) { events.removeFirst(); omitted++; }
            events.addLast(value);
            android.util.Log.i("LibLivePhotoBinding", value);
        }
        synchronized void report(String outcome) {
            Bundle status = new Bundle();
            status.putString("bindingOutcome", outcome);
            status.putString("bindingTrace", "omitted=" + omitted + "\n" + String.join("\n", events));
            sendStatus(1, status);
        }
    }
    private MotionPhotoClient connect(Class<? extends MotionPhotoService> type) throws Exception {
        markPhase("connect "+type.getSimpleName());
        BindingTrace trace = new BindingTrace();
        MotionPhotoClient client=new MotionPhotoClient(getTargetContext(),type,trace);
        try {
            // The unchanged20-second future wait starts after synchronous bindService returns.
            long readyWaitStart = SystemClock.elapsedRealtime();
            client.ready().get(20,TimeUnit.SECONDS);
            long readyWaitElapsed = SystemClock.elapsedRealtime() - readyWaitStart;
            int remotePid = client.remoteProcessId();
            // Reuse the existing PID query; retain its result even if lifecycle logcat is clipped.
            trace.report("ready service=" + type.getSimpleName()
                    + " clientPid=" + android.os.Process.myPid() + " remotePid=" + remotePid
                    + " readyWaitElapsedMs=" + readyWaitElapsed);
            check(remotePid!=android.os.Process.myPid(),"service must actually use another process");
            check(invoke(client,new Request("version"),null,null).success,"remote Rust native library loads");
            return client;
        } catch(Exception | Error error) {
            trace.report("failed-before-close " + error.getClass().getName());
            client.close();
            trace.report("failed-after-close " + error.getClass().getName());
            throw error;
        }
    }
    private void runChecks() throws Exception {
        markPhase("instrumentation running");
        if(!Arrays.asList("all","core","metadata","lifecycle","concurrency","syntax").contains(scope))throw new IllegalArgumentException("unknown acceptance scope");
        byte[] still=asset("cover.jpg"), movie=asset("motion.mp4"), motion=asset("motion-photo.jpg");
        File stillFile=file("own-still.bin",still,0), movieFile=file("own-movie.bin",movie,0), motionFile=file("own-motion-range.bin",motion,13);
        try (ParcelFileDescriptor stillFD=ParcelFileDescriptor.open(stillFile,ParcelFileDescriptor.MODE_READ_ONLY);
             ParcelFileDescriptor movieFD=ParcelFileDescriptor.open(movieFile,ParcelFileDescriptor.MODE_READ_ONLY);
             ParcelFileDescriptor motionFD=ParcelFileDescriptor.open(motionFile,ParcelFileDescriptor.MODE_READ_ONLY);
             MotionPhotoClient parser=connect(MetadataParserService.class);
             MotionPhotoClient builder=connect(MotionPhotoBuilderService.class);
             MotionPhotoClient checker=connect(MotionPhotoCheckerService.class);
             MotionPhotoClient extractor=connect(MotionPhotoExtractorService.class)) {
            if(scenario("syntax")) {
                // Existing Binder actions exercise actual FD ranges and isolated
                // parser processes. Conformance/remux are existing JNI actions,
                // not Binder service capabilities; their transport is explicit.
                NativeEngine engine=new NativeEngine(); AtomicInteger sequence=new AtomicInteger();
                assertions+=SyntaxContracts.run(name->asset("syntax/"+name),(json,p,m)->{
                    String action=new org.json.JSONObject(json).getString("action");
                    MotionPhotoClient selected="parse".equals(action)?parser:"build".equals(action)?builder:"extract".equals(action)||"agtm".equals(action)?extractor:null;
                    if(selected==null) {
                        markPhase("syntax JNI "+action+" pid="+android.os.Process.myPid());
                        return engine.executeJson(json,Input.bytes(p),m.length==0?null:Input.bytes(m),(phase,n,total)->{});
                    }
                    markPhase("syntax Binder "+action+" remotePid="+selected.remoteProcessId());
                    int serial=sequence.incrementAndGet();
                    File primaryFile=file("own-syntax-primary-"+serial,p,13),secondaryFile=file("own-syntax-movie-"+serial,m,17);
                    try(ParcelFileDescriptor primaryFD=ParcelFileDescriptor.open(primaryFile,ParcelFileDescriptor.MODE_READ_ONLY);
                        ParcelFileDescriptor secondaryFD=ParcelFileDescriptor.open(secondaryFile,ParcelFileDescriptor.MODE_READ_ONLY)) {
                        Os.lseek(primaryFD.getFileDescriptor(),3,OsConstants.SEEK_SET);
                        NativeResult value=invoke(selected,SyntaxContracts.request(json),MotionPhotoClient.Source.range(primaryFD,13,p.length),m.length==0?null:MotionPhotoClient.Source.range(secondaryFD,17,m.length));
                        if(Os.lseek(primaryFD.getFileDescriptor(),0,OsConstants.SEEK_CUR)!=3)throw new AssertionError("syntax source cursor changed");
                        return value;
                    } finally { primaryFile.delete();secondaryFile.delete(); }
                },"android-actual-jni-and-binder");
            }
            Os.lseek(motionFD.getFileDescriptor(),7,OsConstants.SEEK_SET);
            MotionPhotoClient.Source source=MotionPhotoClient.Source.range(motionFD,13,motion.length);
            if(scenario("core")) {
            AtomicInteger readProgress=new AtomicInteger();
            markPhase("isolated metadata pipe/read progress");
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
            }
            if(scenario("lifecycle")) {
            for (int i=0;i<8;i++) check(invoke(parser,new Request("parse"),source,null).success,"repeated service response/descriptor lifetime");
            }
            if(scenario("concurrency")) {
            markPhase("concurrent reads and queued service operations");
            List<MotionPhotoClient.Operation> concurrent=new ArrayList<>();
            for(int i=0;i<3;i++) concurrent.add(parser.execute(new Request("parse"),source,null,null));
            for(MotionPhotoClient.Operation operation:concurrent) check(operation.result().get(40,TimeUnit.SECONDS).success,"concurrent bounded pipe readers and service queue drain");
            check(Os.lseek(motionFD.getFileDescriptor(),0,OsConstants.SEEK_CUR)==7,"concurrent reads preserve caller cursor");
            }
            if(scenario("metadata")) {
            markPhase("metadata binding assets");
            byte[] agtmBytes=asset("bindings/agtm-video.mp4"), rawAgtm=asset("bindings/agtm-raw.bin"), collectionBytes=asset("bindings/collection.json");
            File agtmFile=file("own-agtm.bin",agtmBytes,11),collectionFile=file("own-collection.bin",collectionBytes,0);
            try(ParcelFileDescriptor agtmFD=ParcelFileDescriptor.open(agtmFile,ParcelFileDescriptor.MODE_READ_ONLY);
                ParcelFileDescriptor collectionFD=ParcelFileDescriptor.open(collectionFile,ParcelFileDescriptor.MODE_READ_ONLY)) {
                MotionPhotoClient.Source agtmSource=MotionPhotoClient.Source.range(agtmFD,11,agtmBytes.length);
                NativeResult agtm=invoke(parser,new Request("agtm").option("videoInput",true).time(new ExactTime(1,1)),agtmSource,null);
                check(agtm.success&&agtm.files.size()==1&&Arrays.equals(rawAgtm,agtm.files.get(0).copyBytes()),"actual isolated AGTM service raw byte extraction");
                check(!invoke(extractor,new Request("agtm").option("videoInput",true).time(new ExactTime(13,16)),agtmSource,null).success,"service AGTM exact time rejection");
                NativeResult classified=invoke(parser,new Request("classify-metadata"),MotionPhotoClient.Source.range(collectionFD,0,collectionBytes.length),null);
                check(classified.success&&Boolean.TRUE.equals(classified.metadata.get("declaresMotion"))&&Boolean.FALSE.equals(classified.metadata.get("verifiedMotion")),"serialized metadata declaration is not source validation");
            } finally { agtmFile.delete();collectionFile.delete(); }
            }
            if(scenario("concurrency")) {
            // A large FD read keeps cancellation observable without replacing
            // the core with a delay/mock service. No private media is used.
            File large=new File(getTargetContext().getCacheDir(),"own-cancel.bin");
            try (RandomAccessFile fill=new RandomAccessFile(large,"rw")) { fill.setLength(32L*1024*1024); }
            try (ParcelFileDescriptor fd=ParcelFileDescriptor.open(large,ParcelFileDescriptor.MODE_READ_ONLY)) {
                MotionPhotoClient.Operation pending=parser.execute(new Request("parse"),MotionPhotoClient.Source.whole(fd),null,null);
                markPhase("cancel actual service read");
                check(pending.cancel(),"cancel an actual service read immediately");
                try { pending.result().get(5,TimeUnit.SECONDS); throw new AssertionError("cancel returned success"); }
                catch (CancellationException expected) { assertions++; }
                check(Os.fstat(fd.getFileDescriptor()).st_size==32L*1024*1024,"cancel keeps caller-owned FD open");
                markPhase("concurrent cancellation versus completion");
                MotionPhotoClient.Operation race=parser.execute(new Request("version"),null,null,null);
                AtomicInteger winners=new AtomicInteger();
                CountDownLatch startCancel=new CountDownLatch(1);
                Runnable cancel=()->{try{startCancel.await();if(race.cancel())winners.incrementAndGet();}catch(InterruptedException error){Thread.currentThread().interrupt();}};
                Thread firstCancel=new Thread(cancel),secondCancel=new Thread(cancel);
                firstCancel.start();secondCancel.start();startCancel.countDown();firstCancel.join(5000);secondCancel.join(5000);
                check(!firstCancel.isAlive()&&!secondCancel.isAlive(),"concurrent cancellation returns without blocking");
                check(winners.get()<=1,"at most one caller wins terminal cancellation");
                if(winners.get()==1) {
                    try { race.result().get(5,TimeUnit.SECONDS); throw new AssertionError("winning cancellation returned success"); }
                    catch (CancellationException expected) { assertions++; }
                } else check(race.result().get(40,TimeUnit.SECONDS).success,"completion may legitimately win before cancellation");
            } finally { large.delete(); }
            }
        } finally { stillFile.delete();movieFile.delete();motionFile.delete(); }
    }
}
