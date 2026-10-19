package org.liblivephoto;
import java.util.*;
/** Exact signed ticks and positive unsigned 32-bit scale. Never a double. */
public final class ExactTime {
  public final long value; public final long timescale;
  public ExactTime(long value,long timescale){if(timescale<1||timescale>0xffffffffL)throw new IllegalArgumentException("invalid timescale");this.value=value;this.timescale=timescale;}
  Map<String,Object> json(){Map<String,Object> m=new LinkedHashMap<>();m.put("value",Long.toString(value));m.put("timescale",timescale);return m;}
}
