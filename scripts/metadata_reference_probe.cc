// Test adapter to the pinned actual MetadataEngine, including ordinary images.
#include "motion_photo/metadata_engine.h"
#include "motion_photo/motion_photo_utils.h"
#include "image_io/base/message_handler.h"
#include "image_io/base/string_ref_data_source.h"
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
using namespace libmotionphoto::motion_photo;
std::string Text(const std::string& s) {std::string out="\"";const char* hex="0123456789abcdef";for(unsigned char c:s){if(c=='"'||c=='\\'){out+='\\';out+=c;}else if(c<32){out+="\\u00";out+=hex[c>>4];out+=hex[c&15];}else out+=c;}return out+'"';}
int main(int argc,char** argv) {
  if(argc!=4)return 2;
  std::ifstream file(argv[1],std::ios::binary);std::string bytes((std::istreambuf_iterator<char>(file)),{});
  photos_editing_formats::image_io::MessageHandler messages;messages.SetMessageWriter(nullptr);
  MetadataEngine engine(&messages);HandlerOptions options;
  std::string filter=argv[3];if(filter=="none")options.disable_3p_plugins=true;else if(filter!="all")options.enabled_3p_plugins.push_back(filter);
  auto type=GetFileTypeFromFileName(argv[1]);
  photos_editing_formats::image_io::StringRefDataSource source(bytes);
  auto collection=engine.Parse(&source,bytes.size(),type,options);
  const bool declaration=MetadataEngine::IsMotionPhoto(collection,options);
  const bool extracted=engine.ExtractMetadata(argv[1],argv[2],options);
  std::string blocks="[";
  for(int i=0;i<collection.blocks_size();i++) {
    const auto& b=collection.blocks(i);MetadataCollection single;*single.add_blocks()=b;
    if(i)blocks+=',';
    blocks+="{\"type\":"+std::to_string(b.type())+",\"formatIdentifier\":"+Text(b.format_identifier())+
      ",\"offset\":"+Text(std::to_string(b.offset()))+",\"rawBytes\":"+std::to_string(b.raw_bytes().size())+
      ",\"declaresMotion\":"+(MetadataEngine::IsMotionPhoto(single,options)?"true":"false")+
      ",\"videoLength\":"+(b.has_video_length()?Text(std::to_string(b.video_length())):"null")+"}";
  }
  std::cout<<"METADATA_REFERENCE_JSON\t{\"declaresMotion\":"<<(declaration?"true":"false")
    <<",\"extracted\":"<<(extracted?"true":"false")<<",\"blocks\":"<<blocks<<"]}\n";
}
