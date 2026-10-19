package org.liblivephoto;
import java.io.*;
import java.nio.file.Path;
import java.util.concurrent.*;
import java.util.concurrent.atomic.*;
/** Java/JNI client. No hidden file publication, uploads or background service. */
public final class NativeEngine {
  public static final int MAX_INPUT_BYTES=128*1024*1024;
  private static volatile boolean loaded;
  public interface Progress {void update(String phase,long completed,long total);}
  public interface Completion {void complete(NativeResult result,Throwable error);}
  private static synchronized void ensureLoaded(){if(!loaded){String path=System.getProperty("liblivephoto.native.path");if(path==null)System.loadLibrary("livephoto_native");else System.load(path);loaded=true;}}
  private static native byte[] nativeProcess(String request,byte[] primary,byte[] movie);
  public NativeResult execute(Request request,Input primary,Input movie)throws IOException{return executeJson(request.toJson(),primary,movie,(p,n,t)->{});}
  /** Raw structured request uses the same maintained schema as CLI/C/Swift. */
  public NativeResult executeJson(String request,Input primary,Input movie,Progress progress)throws IOException {
    ensureLoaded();if(request.length()>1024*1024)throw new IOException("request budget exceeded");byte[] still=primary==null?new byte[0]:primary.snapshot(MAX_INPUT_BYTES,progress);byte[] video=movie==null?new byte[0]:movie.snapshot(MAX_INPUT_BYTES-still.length,progress);if(still.length>MAX_INPUT_BYTES||video.length>MAX_INPUT_BYTES-still.length)throw new IOException("input budget exceeded");progress.update("native",-1,-1);byte[] result=nativeProcess(request,still,video);if(result==null)throw new IOException("native returned null without a JVM exception");NativeResult decoded=NativeResult.decode(result);progress.update("complete",1,1);return decoded;
  }
  public NativeResult version()throws IOException{return execute(new Request("version"),null,null);}
  public NativeResult parse(Input input)throws IOException{return execute(new Request("parse"),input,null);}
  public NativeResult check(Input input)throws IOException{return execute(new Request("check"),input,null);}
  public NativeResult metadata(Input input)throws IOException{return execute(new Request("metadata"),input,null);}
  public NativeResult classifyMetadata(Input serialized)throws IOException{return execute(new Request("classify-metadata"),serialized,null);}
  public NativeResult extract(Input input,boolean still,boolean video,boolean xmp)throws IOException{return execute(new Request("extract").option("still",still).option("video",video).option("xmp",xmp),input,null);}
  public NativeResult build(Input still,Input movie,ExactTime time,String target)throws IOException {Request r=new Request("build").inputKind("parts").target(target);if(time!=null)r.time(time);return execute(r,still,movie);}
  public NativeResult agtm(Input input,ExactTime time,boolean standaloneVideo)throws IOException{Request r=new Request("agtm").option("videoInput",standaloneVideo);if(time!=null)r.time(time);return execute(r,input,null);}
  public Operation submit(Executor executor,Request request,Input primary,Input movie,Progress progress,Completion completion){final String immutableRequest=request.toJson();Operation op=new Operation(completion);try{executor.execute(()->{if(!op.start())return;try{NativeResult r=executeJson(immutableRequest,primary,movie,(p,n,t)->{if(op.cancelled.get())throw new CancellationException("operation cancelled");progress.update(p,n,t);});op.finish(r,null);}catch(Throwable e){op.finish(null,e);}});}catch(RuntimeException e){op.finish(null,e);}return op;}
  /** Read cancellation stops at chunk callbacks. Synchronous native work cannot
   * be interrupted within its phase; late results are discarded after cancel. */
  public static final class Operation {
    public enum State {PENDING,RUNNING,COMPLETED,FAILED,CANCELLED}
    private final AtomicReference<State> state=new AtomicReference<>(State.PENDING);private final AtomicBoolean terminal=new AtomicBoolean(),cancelled=new AtomicBoolean();private final Completion completion;
    Operation(Completion c){completion=c;}public State state(){return state.get();}boolean start(){return state.compareAndSet(State.PENDING,State.RUNNING);}
    public boolean cancel(){if(!terminal.compareAndSet(false,true))return false;cancelled.set(true);state.set(State.CANCELLED);completion.complete(null,new CancellationException("operation cancelled"));return true;}
    void finish(NativeResult result,Throwable error){if(terminal.compareAndSet(false,true)){state.set(error==null&&result!=null&&result.success?State.COMPLETED:State.FAILED);completion.complete(result,error);}}
  }
}
