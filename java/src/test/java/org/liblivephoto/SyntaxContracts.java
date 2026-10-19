package org.liblivephoto;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Acceptance-only reader; all format operations execute Rust through the caller. */
public final class SyntaxContracts {
    public interface Reader { byte[] read(String name) throws Exception; }
    public interface Executor { NativeResult execute(String request, byte[] primary, byte[] movie) throws Exception; }
    public interface CaseObserver { void passed(String id, int fields, int files, String manifestSha256); }
    private static void check(boolean value, String context) { if (!value) throw new AssertionError(context); }
    private static String digest(byte[] bytes) throws Exception {
        StringBuilder text = new StringBuilder();
        for (byte b : MessageDigest.getInstance("SHA-256").digest(bytes)) text.append(String.format(Locale.ROOT, "%02x", b & 255));
        return text.toString();
    }
    private static byte[] data(Object source, Reader reader, Map<String,NativeResult> previous) throws Exception {
        if (source == null) return new byte[0];
        if (source instanceof String) return reader.read((String)source);
        Map<?,?> reference = (Map<?,?>)source;
        int index = reference.containsKey("file") ? ((Number)reference.get("file")).intValue() : 0;
        return previous.get(reference.get("case")).files.get(index).copyBytes();
    }
    @SuppressWarnings("unchecked")
    public static int run(Reader reader, Executor executor, String label) throws Exception {
        return run(reader,executor,label,(id,fields,files,sha)->{});
    }
    public static int run(Reader reader, Executor executor, String label, CaseObserver observer) throws Exception {
        byte[] manifestBytes = reader.read("cases.json");
        String manifestDigest = digest(manifestBytes);
        Map<?,?> manifest = (Map<?,?>)Json.parse(new String(manifestBytes,StandardCharsets.UTF_8));
        for (Map.Entry<?,?> entry : ((Map<?,?>)manifest.get("sha256")).entrySet())
            check(digest(reader.read((String)entry.getKey())).equals(entry.getValue()), "fixture hash "+entry.getKey());
        Map<String,NativeResult> previous = new LinkedHashMap<>();
        int cases = 0;
        for (Object item : (List<?>)manifest.get("cases")) {
            Map<?,?> c = (Map<?,?>)item;
            String id = (String)c.get("id");
            NativeResult result = executor.execute(Json.write(c.get("request")),data(c.get("primary"),reader,previous),data(c.get("movie"),reader,previous));
            check(result.success == (Boolean)c.get("success"), id+": status "+result.headerJson);
            Map<String,Object> root = new LinkedHashMap<>(); root.put("success",result.success); root.put("metadata",result.metadata);
            for (Object rawFact : (List<?>)c.get("expect")) {
                Map<?,?> fact = (Map<?,?>)rawFact;
                Object value = root;
                for (Object component : (List<?>)fact.get("path")) value = component instanceof Number ? ((List<?>)value).get(((Number)component).intValue()) : ((Map<?,?>)value).get(component);
                check(Objects.equals(value,fact.get("value")),id+": "+fact.get("path")+" actual="+value+" expected="+fact.get("value"));
            }
            List<?> files = c.containsKey("files") ? (List<?>)c.get("files") : Collections.emptyList();
            check(result.files.size()==files.size(),id+": file count");
            for (int i=0;i<files.size();i++) {
                Map<?,?> expected = (Map<?,?>)files.get(i); NativeResult.OutputFile file = result.files.get(i);
                check(file.mime.equals(expected.get("mime")),id+": MIME");
                check(Arrays.equals(file.copyBytes(),data(expected.get("fixture"),reader,previous)),id+": original/golden media bytes");
            }
            previous.put(id,result); cases++;
            System.out.println("SYNTAX_CONTRACT_CASE "+label+" "+id+" fields="+((List<?>)c.get("expect")).size()+" files="+files.size());
            observer.passed(id,((List<?>)c.get("expect")).size(),files.size(),manifestDigest);
        }
        check(cases==42,"complete syntax suite required");
        System.out.println("SYNTAX_CONTRACTS_PASSED entry="+label+" cases="+cases+" manifest_sha256="+manifestDigest);
        return cases;
    }

    /** Existing public Request API, for the actions already exposed over Binder. */
    public static Request request(String json) throws Exception {
        Map<?,?> values = (Map<?,?>)Json.parse(json);
        Request request = new Request((String)values.get("action"));
        for (Map.Entry<?,?> entry : values.entrySet()) {
            String key = (String)entry.getKey(); Object value = entry.getValue();
            if (key.equals("action")) continue;
            if (key.equals("time")) {
                Map<?,?> t = (Map<?,?>)value;
                request.time(new ExactTime(Long.parseLong((String)t.get("value")),((Number)t.get("timescale")).longValue()));
            } else if (value instanceof String) request.option(key,(String)value);
            else if (value instanceof Boolean) request.option(key,(Boolean)value);
            else throw new IllegalArgumentException("unsupported Binder acceptance request field: "+key);
        }
        return request;
    }

    public static void main(String[] args) throws Exception {
        Path directory = Paths.get(args[0]); NativeEngine engine = new NativeEngine();
        run(name->Files.readAllBytes(directory.resolve(name)),(request,p,m)->engine.executeJson(request,Input.bytes(p),m.length==0?null:Input.bytes(m),(phase,n,total)->{}),"packaged-jar-jni");
    }
}
