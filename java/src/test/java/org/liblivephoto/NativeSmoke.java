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
    NativeResult format=engine.execute(new Request("conformance").option("profile","google-video-container-1.0").tracks(1),Input.bytes(movie),null);
    check("google-video-container-1.0-declarations".equals(format.metadata.get("profile"))&&format.metadata.get("checkedRules") instanceof List&&format.files.isEmpty(),"explicit native conformance report without format-wide certification");
    check(!engine.execute(new Request("conformance").option("profile","google-video-container-1.0").tracks(1,1),Input.bytes(movie),null).success,"ambiguous primary/secondary association rejected");
    AtomicReference<Runnable> pending=new AtomicReference<>();AtomicInteger callbacks=new AtomicInteger();NativeEngine.Operation op=engine.submit(pending::set,new Request("parse"),Input.bytes(motion),null,(p,n,t)->{},(r,e)->callbacks.incrementAndGet());check(op.cancel(),"cancel pending");pending.get().run();check(callbacks.get()==1&&op.state()==NativeEngine.Operation.State.CANCELLED,"terminal cancellation");
    for(int i=0;i<32;i++){NativeResult loop=engine.parse(Input.bytes(motion));check(loop.success,"repeated JNI response lifetime");}
    ByteArrayOutputStream wire=new ByteArrayOutputStream();extracted.writeEnvelope(wire);NativeResult transported=NativeResult.readEnvelope(new ByteArrayInputStream(wire.toByteArray()),wire.size());check(transported.success&&Arrays.equals(movie,transported.files.get(0).copyBytes()),"streamed response preserves original payload");
    try{NativeResult.readEnvelope(new ByteArrayInputStream(wire.toByteArray()),16);throw new AssertionError("stream budget ignored");}catch(IOException expected){}
    wire.reset();NativeResult.failure("service error \"quoted\" ").writeEnvelope(wire);check(!NativeResult.decode(wire.toByteArray()).success,"structured transport failure");
    Path bindings=demo.getParent().getParent().getParent().resolve("fixtures/bindings");byte[] rawAgtm=Files.readAllBytes(bindings.resolve("agtm-raw.bin"));
    NativeResult album=engine.metadata(Input.path(bindings.resolve("camera-album.jpg")));check(album.success&&Boolean.FALSE.equals(album.metadata.get("verifiedMotion")),"camera footer alone is not verified motion");boolean foundAlbum=false;
    for(Object decoded:(List<?>)album.metadata.get("decoded")){Map<?,?> item=(Map<?,?>)decoded;if("camera_album".equals(item.get("provider"))){Map<?,?> value=(Map<?,?>)item.get("value");check("31".equals(value.get("imageTime"))&&Boolean.FALSE.equals(value.get("pairingVerified"))&&String.valueOf(value.get("rawJson")).contains("1.234567890123456789e100"),"raw footer/time/identity projection across JNI");foundAlbum=true;}}
    check(foundAlbum,"bounded camera footer provider across JNI");
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
    NativeResult vivo=engine.metadata(Input.bytes(Files.readAllBytes(bindings.resolve("vivo-stream-motion.jpg"))));check(vivo.success&&Boolean.TRUE.equals(vivo.metadata.get("verifiedMotion")),"vivo vector actual movie verification");boolean foundVivo=false;
    for(Object item:(List<?>)vivo.metadata.get("decoded")){Map<?,?> decoded=(Map<?,?>)item;if("vivo_media".equals(decoded.get("provider"))){Map<?,?> value=(Map<?,?>)decoded.get("value");List<?> streams=(List<?>)value.get("streams");check(streams.size()==3&&Boolean.TRUE.equals(value.get("duplicateKeys"))&&Boolean.FALSE.equals(value.get("streamSemanticsInterpreted"))&&Boolean.FALSE.equals(value.get("pairingVerified")),"ordered opaque streams without semantic or pair inference");Map<?,?> first=(Map<?,?>)streams.get(0);int index=((Number)first.get("blockIndex")).intValue();Map<?,?> block=(Map<?,?>)((List<?>)vivo.metadata.get("blocks")).get(index);check("00ff6f7061717565".equals(block.get("rawBytesHex")),"vivo stream byte identity through JNI");foundVivo=true;}}
    check(foundVivo,"vivo provider available through maintained JNI metadata");
    Path syntax=bindings.getParent().resolve("syntax");
    NativeResult icc=engine.execute(new Request("conformance").option("profile","icc-structure-v4.4"),Input.bytes(Files.readAllBytes(syntax.resolve("icc-structure-v4.4.icc"))),null);
    check(icc.success&&"icc.1-2022-v4.4-structure".equals(icc.metadata.get("profile")),"scoped ICC structure across JNI");
    NativeResult unknownIcc=engine.execute(new Request("conformance").option("profile","icc-structure-v4.4"),Input.path(syntax.resolve("icc-unsupported-v4.3.icc")),null);
    check(!unknownIcc.success&&unknownIcc.headerJson.contains("Unsupported"),"unsupported ICC version is not a structure pass");
    byte[] iccRaw=Files.readAllBytes(syntax.resolve("icc-structure-v4.4.icc"));
    check(!engine.execute(new Request("conformance").option("profile","icc-structure-v4.4"),Input.bytes(Arrays.copyOf(iccRaw,iccRaw.length-1)),null).success,"truncated ICC fails explicitly");
    byte[] pceMovie=Files.readAllBytes(syntax.resolve("aac-pce-wave.mov"));
    NativeResult pce=engine.execute(new Request("parse").inputKind("parts"),Input.bytes(still),Input.bytes(pceMovie));
    check(pce.success,"actual PCE/wave owned movie parse");boolean foundPce=false;
    for(Object row:(List<?>)pce.metadata.get("sampleDescriptions")){Map<?,?> desc=(Map<?,?>)row;Object value=desc.get("audio");if(value instanceof Map){Map<?,?> config=(Map<?,?>)((Map<?,?>)value).get("aac");check(((Number)config.get("channels")).intValue()==2&&((Number)config.get("sampleRate")).intValue()==48000&&((Number)config.get("channelConfiguration")).intValue()==0,"PCE channel/rate facts across JNI");Map<?,?> program=(Map<?,?>)config.get("programConfig");check(Boolean.FALSE.equals(program.get("speakerPositionsInferred"))&&Boolean.FALSE.equals(program.get("renderingVerified")),"PCE facts are not rendered speaker placement");foundPce=true;}}
    check(foundPce,"actual AAC PCE projection present");
    byte[] badPce=pceMovie.clone();int esdsMarkers=0;for(int i=0;i+4<badPce.length;i++){if(badPce[i]=='e'&&badPce[i+1]=='s'&&badPce[i+2]=='d'&&badPce[i+3]=='s'){badPce[i+4]=1;esdsMarkers++;}}
    check(esdsMarkers==1,"owned fixture has one framed esds");
    NativeResult rejectedEsds=engine.execute(new Request("conformance").option("profile","google-video-container-1.0").tracks(1),Input.bytes(badPce),null);
    check(!rejectedEsds.success&&rejectedEsds.headerJson.contains("esds version/flags"),"unknown esds version rejected across JNI");
    NativeResult pceBuilt=engine.build(Input.bytes(still),Input.bytes(pceMovie),new ExactTime(1,10),"google-jpeg");check(pceBuilt.success,"PCE container lossless build");
    check(Arrays.equals(pceMovie,engine.extract(Input.bytes(pceBuilt.files.get(0).copyBytes()),false,true,false).files.get(0).copyBytes()),"PCE movie bytes retained across JNI build/extract");
    SyntaxContracts.run(name->Files.readAllBytes(("contract-suites.json".equals(name)?syntax.getParent():syntax).resolve(name)),(request,p,m)->engine.executeJson(request,Input.bytes(p),m.length==0?null:Input.bytes(m),(phase,n,total)->{}),"host-jni");
    Path vendor=bindings.getParent().resolve("vendor");
    SyntaxContracts.run(name->Files.readAllBytes(("contract-suites.json".equals(name)?vendor.getParent():vendor).resolve(name)),(request,p,m)->engine.executeJson(request,Input.bytes(p),m.length==0?null:Input.bytes(m),(phase,n,total)->{}),"vendor-host-jni",(id,fields,files,sha)->{},13);
    Path selection=bindings.getParent().resolve("vendor-selection");
    SyntaxContracts.run(name->Files.readAllBytes(("contract-suites.json".equals(name)?selection.getParent():selection).resolve(name)),(request,p,m)->engine.executeJson(request,Input.bytes(p),m.length==0?null:Input.bytes(m),(phase,n,total)->{}),"vendor-selection-host-jni",(id,fields,files,sha)->{},20);
    System.out.println("JNI_SMOKE passed: existing regressions plus all42 new syntax/preservation/exact-time cases");
  }
}
