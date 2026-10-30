package org.liblivephoto;
import java.io.IOException;
import java.math.BigDecimal;
import java.util.*;
/** Small bounded protocol JSON codec. Numbers stay exact BigDecimal values. */
final class Json {
  static Object parse(String text) throws IOException {if(text.length()>64*1024*1024)throw new IOException("JSON length limit");Reader r=new Reader(text);Object v=r.value(0);r.ws();if(r.p!=text.length())throw new IOException("trailing JSON");return v; }
  static String write(Object value) {StringBuilder s=new StringBuilder();write(value,s);return s.toString();}
  private static void write(Object v,StringBuilder s) {
    if(v==null){s.append("null");return;}if(v instanceof String){quote((String)v,s);return;}if(v instanceof Boolean||v instanceof Number){s.append(v);return;}
    if(v instanceof Map){s.append('{');boolean first=true;for(Object e:((Map<?,?>)v).entrySet()){Map.Entry<?,?> item=(Map.Entry<?,?>)e;if(!first)s.append(',');first=false;quote((String)item.getKey(),s);s.append(':');write(item.getValue(),s);}s.append('}');return;}
    if(v instanceof Iterable){s.append('[');boolean first=true;for(Object item:(Iterable<?>)v){if(!first)s.append(',');first=false;write(item,s);}s.append(']');return;}throw new IllegalArgumentException("unsupported JSON value");
  }
  private static void quote(String value,StringBuilder s){s.append('"');for(int i=0;i<value.length();i++){char c=value.charAt(i);switch(c){case '"':s.append("\\\"");break;case '\\':s.append("\\\\");break;case '\n':s.append("\\n");break;case '\r':s.append("\\r");break;case '\t':s.append("\\t");break;default:if(c<32||Character.isSurrogate(c)){s.append(String.format(Locale.ROOT,"\\u%04x",(int)c));}else s.append(c);}}s.append('"');}
  private static final class Reader {
    final String s;int p,nodes;Reader(String s){this.s=s;}void ws(){while(p<s.length()&&" \r\n\t".indexOf(s.charAt(p))>=0)p++;}
    Object value(int depth)throws IOException {if(depth>128)throw new IOException("JSON depth limit");if(++nodes>1048576)throw new IOException("JSON node limit");ws();if(p>=s.length())throw new IOException("truncated JSON");char c=s.charAt(p);
      if(c=='"')return string();if(c=='{'){p++;Map<String,Object> out=new LinkedHashMap<>();ws();if(take('}'))return out;do{ws();String k=string();ws();need(':');Object v=value(depth+1);if(out.containsKey(k))throw new IOException("duplicate JSON key");out.put(k,v);ws();if(take('}'))return out;need(',');}while(true);}
      if(c=='['){p++;List<Object> out=new ArrayList<>();ws();if(take(']'))return out;do{out.add(value(depth+1));ws();if(take(']'))return out;need(',');}while(true);}
      for(String literal:new String[]{"true","false","null"})if(s.startsWith(literal,p)){p+=literal.length();return literal.equals("null")?null:Boolean.valueOf(literal);}
      int start=p;while(p<s.length()&&"-+0123456789.eE".indexOf(s.charAt(p))>=0){if(p-start>=1024)throw new IOException("JSON number length limit");p++;}if(p==start)throw new IOException("invalid JSON value");String n=s.substring(start,p);if(!n.matches("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?"))throw new IOException("invalid JSON number");try{return new BigDecimal(n);}catch(NumberFormatException e){throw new IOException("invalid JSON number",e);}
    }
    String string()throws IOException {need('"');StringBuilder out=new StringBuilder();while(p<s.length()){char c=s.charAt(p++);if(c=='"')return out.toString();if(c<32)throw new IOException("control in JSON string");if(c!='\\'){out.append(c);continue;}if(p>=s.length())break;char e=s.charAt(p++);switch(e){case '"':case '\\':case '/':out.append(e);break;case 'n':out.append('\n');break;case 'r':out.append('\r');break;case 't':out.append('\t');break;case 'b':out.append('\b');break;case 'f':out.append('\f');break;case 'u':if(p+4>s.length())throw new IOException("truncated Unicode escape");try{out.append((char)Integer.parseInt(s.substring(p,p+4),16));}catch(NumberFormatException ex){throw new IOException("invalid Unicode escape",ex);}p+=4;break;default:throw new IOException("invalid JSON escape");}}throw new IOException("unterminated JSON string");}
    boolean take(char c){if(p<s.length()&&s.charAt(p)==c){p++;return true;}return false;}void need(char c)throws IOException{if(!take(c))throw new IOException("expected "+c);}
  }
}
