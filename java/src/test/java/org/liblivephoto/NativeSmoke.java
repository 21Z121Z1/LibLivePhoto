package org.liblivephoto;
import java.io.*;import java.nio.channels.FileChannel;import java.nio.file.*;import java.util.*;import java.util.concurrent.atomic.*;
/** Actual JVM/JNI acceptance; deliberately no test framework dependency. */
public final class NativeSmoke {
  private static void check(boolean v,String message){if(!v)throw new AssertionError(message);}
  public static void main(String[] args)throws Exception{
    Path demo=Paths.get(args[0]);NativeEngine engine=new NativeEngine();check(engine.version().success,"native version");byte[] still=Files.readAllBytes(demo.resolve("cover.jpg")),movie=Files.readAllBytes(demo.resolve("motion.mp4"));
    NativeResult built=engine.build(Input.bytes(still),Input.bytes(movie),new ExactTime(13,16),"google-jpeg");check(built.success,"native build: "+built.headerJson);byte[] motion=built.files.get(0).copyBytes();NativeResult parsed=engine.parse(Input.bytes(motion));check(parsed.success,"native parse");check(parsed.headerJson.contains("812500"),"exact parsed photo time");NativeResult extracted=engine.extract(Input.bytes(motion),false,true,false);check(extracted.success&&Arrays.equals(movie,extracted.files.get(0).copyBytes()),"byte-exact JNI extraction");check(engine.metadata(Input.path(demo.resolve("motion-photo.jpg"))).success,"metadata path");
    Path wrapped=Files.createTempFile("liblivephoto-jni-",".bin");try{try(OutputStream out=Files.newOutputStream(wrapped)){out.write(new byte[13]);out.write(motion);out.write(new byte[4]);}try(FileChannel channel=FileChannel.open(wrapped,StandardOpenOption.READ)){channel.position(7);NativeResult range=engine.metadata(Input.range(channel,13,motion.length));check(range.success,"descriptor/range metadata");check(channel.position()==7&&channel.isOpen(),"borrowed descriptor ownership/cursor");}}
    finally{Files.delete(wrapped);}
    NativeResult blocked=engine.build(Input.bytes(still),Input.bytes(movie),new ExactTime(1,3),"google-jpeg");check(!blocked.success&&blocked.files.isEmpty(),"non-exact time must not build");check(!engine.parse(Input.bytes(new byte[]{1,2,3})).success,"invalid input error");
    AtomicReference<Runnable> pending=new AtomicReference<>();AtomicInteger callbacks=new AtomicInteger();NativeEngine.Operation op=engine.submit(pending::set,new Request("parse"),Input.bytes(motion),null,(p,n,t)->{},(r,e)->callbacks.incrementAndGet());check(op.cancel(),"cancel pending");pending.get().run();check(callbacks.get()==1&&op.state()==NativeEngine.Operation.State.CANCELLED,"terminal cancellation");
    for(int i=0;i<32;i++){NativeResult loop=engine.parse(Input.bytes(motion));check(loop.success,"repeated JNI response lifetime");}
    try{NativeResult.decode(new byte[]{1,2,3,4});throw new AssertionError("bad envelope accepted");}catch(IOException expected){}
    System.out.println("JNI_SMOKE passed: version, memory/path/range, exact build/extract/time, diagnostics, cancellation, 32 response lifetimes");
  }
}
