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
    ByteArrayOutputStream wire=new ByteArrayOutputStream();extracted.writeEnvelope(wire);NativeResult transported=NativeResult.readEnvelope(new ByteArrayInputStream(wire.toByteArray()),wire.size());check(transported.success&&Arrays.equals(movie,transported.files.get(0).copyBytes()),"streamed response preserves original payload");
    try{NativeResult.readEnvelope(new ByteArrayInputStream(wire.toByteArray()),16);throw new AssertionError("stream budget ignored");}catch(IOException expected){}
    wire.reset();NativeResult.failure("service error \"quoted\" ").writeEnvelope(wire);check(!NativeResult.decode(wire.toByteArray()).success,"structured transport failure");
    Path bindings=demo.getParent().getParent().getParent().resolve("fixtures/bindings");byte[] rawAgtm=Files.readAllBytes(bindings.resolve("agtm-raw.bin"));
    byte[] agtmMovie=Files.readAllBytes(bindings.resolve("agtm-video.mp4")),agtmMotion=Files.readAllBytes(bindings.resolve("agtm-motion.jpg"));
    Path bounded=Files.createTempFile("liblivephoto-agtm-range-",".bin");try{
      try(OutputStream out=Files.newOutputStream(bounded)){out.write(new byte[19]);out.write(agtmMovie);out.write(new byte[3]);}
      try(FileChannel channel=FileChannel.open(bounded,StandardOpenOption.READ)){
        channel.position(5);
        for(Input source:Arrays.asList(Input.bytes(agtmMovie),Input.path(bindings.resolve("agtm-video.mp4")),Input.range(channel,19,agtmMovie.length))){NativeResult agtm=engine.agtm(source,new ExactTime(1,1),true);check(agtm.success&&agtm.files.size()==1&&Arrays.equals(rawAgtm,agtm.files.get(0).copyBytes()),"AGTM bytes across memory/path/descriptor-range: "+agtm.headerJson);check(agtm.headerJson.contains("renderedPixels\":false")&&agtm.headerJson.contains("encodedSlopeAngle"),"complete AGTM model without render claim");}
        check(channel.position()==5&&channel.isOpen(),"AGTM descriptor cursor/ownership");
      }
    }finally{Files.delete(bounded);}
    NativeResult imageAgtm=engine.agtm(Input.bytes(agtmMotion),null,false);check(imageAgtm.success&&Arrays.equals(rawAgtm,imageAgtm.files.get(0).copyBytes()),"AGTM image's original photo time");
    check(!engine.agtm(Input.bytes(agtmMovie),new ExactTime(13,16),true).success,"AGTM does not round an unrepresentable metadata-track time");
    NativeResult classification=engine.classifyMetadata(Input.path(bindings.resolve("collection.json")));check(classification.success&&Boolean.TRUE.equals(classification.metadata.get("declaresMotion"))&&Boolean.FALSE.equals(classification.metadata.get("verifiedMotion")),"serialized collection declares, never revalidates source");
    check(engine.execute(new Request("metadata").disableThirdParty(true),Input.bytes(agtmMotion),null).success,"provider controls across JNI");
    NativeResult blocks=engine.extractMetadata(Input.path(bindings.resolve("agtm-motion.jpg")));check(blocks.success&&!blocks.files.isEmpty(),"complete raw metadata blocks");boolean foundXmp=false;for(NativeResult.OutputFile file:blocks.files)if(file.mime.equals("application/rdf+xml")){foundXmp=true;check(new String(file.copyBytes(),java.nio.charset.StandardCharsets.UTF_8).contains("MotionPhoto"),"original XMP block");}check(foundXmp,"XMP not lost from metadata extraction");
    try{NativeResult.decode(new byte[]{1,2,3,4});throw new AssertionError("bad envelope accepted");}catch(IOException expected){}
    try{Input.bytes(new byte[NativeEngine.MAX_INPUT_BYTES+1]);throw new AssertionError("oversized retained snapshot accepted");}catch(IllegalArgumentException expected){}
    StringBuilder dense=new StringBuilder("[");for(int i=0;i<1048576;i++)dense.append("null,");dense.append("null]");
    try{Json.parse(dense.toString());throw new AssertionError("JSON node budget ignored");}catch(IOException expected){check(expected.getMessage().contains("node limit"),"aggregate JSON node diagnostic");}
    char[] hugeNumber=new char[1025];Arrays.fill(hugeNumber,'9');
    try{Json.parse(new String(hugeNumber));throw new AssertionError("JSON number length budget ignored");}catch(IOException expected){check(expected.getMessage().contains("number length limit"),"JSON numeric allocation diagnostic");}
    System.out.println("JNI_SMOKE passed: version, memory/path/range, exact build/extract/time, diagnostics, cancellation, 32 response lifetimes, bounded streamed envelope, AGTM image/video/input parity, source-unverified collection classification and provider controls");
  }
}
