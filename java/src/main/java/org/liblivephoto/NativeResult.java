package org.liblivephoto;
import java.io.*;
import java.nio.*;
import java.nio.charset.*;
import java.util.*;
/** Owned operation result, including exact original payload bytes and raw JSON. */
public final class NativeResult {
  public final boolean success; public final String headerJson; public final Map<String,Object> metadata; public final List<OutputFile> files;
  private NativeResult(boolean success,String json,Map<String,Object> metadata,List<OutputFile> files){this.success=success;this.headerJson=json;this.metadata=Collections.unmodifiableMap(metadata);this.files=Collections.unmodifiableList(files);}
  public static final class OutputFile {
    public final String name,mime;public final long resourceId;private final byte[] bytes;
    OutputFile(String name,String mime,long id,byte[] bytes){this.name=name;this.mime=mime;this.resourceId=id;this.bytes=bytes;}
    public ByteBuffer data(){return ByteBuffer.wrap(bytes).asReadOnlyBuffer();}
    public byte[] copyBytes(){return bytes.clone();}
    public void writeTo(OutputStream output)throws IOException{output.write(bytes);}
    /** CREATE_NEW; no original/existing path may be overwritten. */
    public void writeNew(java.nio.file.Path path)throws IOException{try(OutputStream output=java.nio.file.Files.newOutputStream(path,java.nio.file.StandardOpenOption.CREATE_NEW,java.nio.file.StandardOpenOption.WRITE)){output.write(bytes);}}
  }
  /** Transport the owned LLP1 response through a pipe, outside Binder parcels. */
  public void writeEnvelope(OutputStream output)throws IOException {
    DataOutputStream out=new DataOutputStream(output);out.writeInt(0x4c4c5031);writeField(out,headerJson.getBytes(StandardCharsets.UTF_8));out.writeInt(files.size());
    for(OutputFile file:files){writeField(out,file.mime.getBytes(StandardCharsets.UTF_8));writeField(out,file.name.getBytes(StandardCharsets.UTF_8));out.writeLong(file.resourceId);writeField(out,file.bytes);}out.flush();
  }
  private static void writeField(DataOutputStream out,byte[] bytes)throws IOException{out.writeInt(bytes.length);out.write(bytes);}
  /** Bounded stream read; the caller owns and closes the stream. */
  public static NativeResult readEnvelope(InputStream input,int budget)throws IOException {
    if(budget<16||budget>320*1024*1024)throw new IOException("invalid response budget");ByteArrayOutputStream bytes=new ByteArrayOutputStream();byte[] chunk=new byte[65536];int total=0,n;
    while((n=input.read(chunk))!=-1){if(n==0)throw new IOException("response read made no progress");if(n>budget-total)throw new IOException("response budget exceeded");bytes.write(chunk,0,n);total+=n;}return decode(bytes.toByteArray());
  }
  public static NativeResult failure(String message){Map<String,Object> metadata=new LinkedHashMap<>();metadata.put("error",message);Map<String,Object> root=new LinkedHashMap<>();root.put("success",false);root.put("metadata",metadata);return new NativeResult(false,Json.write(root),metadata,new ArrayList<OutputFile>());}
  private static byte[] field(DataInputStream in,int max)throws IOException {int length=in.readInt();if(length<0||length>max||length>in.available())throw new IOException("invalid native response field");byte[] bytes=new byte[length];in.readFully(bytes);return bytes;}
  private static String text(byte[] bytes)throws IOException{return StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString();}
  @SuppressWarnings("unchecked")
  public static NativeResult decode(byte[] response)throws IOException {
    if(response.length>320*1024*1024)throw new IOException("native response budget exceeded");DataInputStream in=new DataInputStream(new ByteArrayInputStream(response));if(in.readInt()!=0x4c4c5031)throw new IOException("native protocol mismatch");String header=text(field(in,64*1024*1024));Object decoded=Json.parse(header);if(!(decoded instanceof Map))throw new IOException("invalid native header");Map<String,Object> root=(Map<String,Object>)decoded;if(!(root.get("success") instanceof Boolean)||!(root.get("metadata") instanceof Map))throw new IOException("native result fields missing");int count=in.readInt();if(count<0||count>4096)throw new IOException("native file count limit");List<OutputFile> files=new ArrayList<>();for(int i=0;i<count;i++){String mime=text(field(in,4096)),name=text(field(in,4096));long id=in.readLong();if(id< -1)throw new IOException("invalid native resource ID");files.add(new OutputFile(name,mime,id,field(in,256*1024*1024)));}if(in.available()!=0)throw new IOException("trailing native envelope bytes");return new NativeResult((Boolean)root.get("success"),header,(Map<String,Object>)root.get("metadata"),files);
  }
}
