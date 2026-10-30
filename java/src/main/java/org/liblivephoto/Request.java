package org.liblivephoto;
import java.util.*;
/** Maintained operation request; JSON escape/number handling is shared and exact. */
public final class Request {
  private final Map<String,Object> values=new LinkedHashMap<>();
  public Request(String action){values.put("action",Objects.requireNonNull(action));}
  public Request time(ExactTime time){values.put("time",time.json());return this;}
  public Request inputKind(String kind){return option("inputKind",kind);}
  public Request target(String profile){return option("target",profile);}
  public Request pairIdentifier(String identifier){return option("pairId",identifier);}
  public Request disableThirdParty(boolean disabled){return option("disableThirdParty",disabled);}
  public Request enabledProviders(String... names){values.put("enabledProviders",Arrays.asList(names.clone()));return this;}
  public Request tracks(long... ids){List<Long> out=new ArrayList<>();for(long id:ids){if(id<1||id>0xffffffffL)throw new IllegalArgumentException("positive track ID required");out.add(id);}values.put("trackIds",out);return this;}
  public Request option(String key,String value){values.put(key,Objects.requireNonNull(value));return this;}
  public Request option(String key,boolean value){values.put(key,value);return this;}
  public String toJson(){return Json.write(values);}
}
