// Test adapter to the pinned actual MetadataEngine, including ordinary images.
#include "motion_photo/metadata_engine.h"
#include "motion_photo/motion_photo_utils.h"
#include "legacy_micro_video_metadata.pb.h"
#include "oplus_olive_metadata.pb.h"
#include "samsung_sef_metadata.pb.h"
#include "honor_live_photo_metadata.pb.h"
#include "image_io/base/message_handler.h"
#include "image_io/base/string_ref_data_source.h"
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
using namespace libmotionphoto::motion_photo;
std::string Text(const std::string& s) {std::string out="\"";const char* hex="0123456789abcdef";for(unsigned char c:s){if(c=='"'||c=='\\'){out+='\\';out+=c;}else if(c<32){out+="\\u00";out+=hex[c>>4];out+=hex[c&15];}else out+=c;}return out+'"';}
std::string Semantic(const MetadataBlock& b) {
  const auto& type=b.payload_type_url();const auto& bytes=b.decoded_payload_bytes();
  if(type.find("LegacyMicroVideoMetadata")!=std::string::npos) {
    LegacyMicroVideoMetadata m;if(!m.ParseFromString(bytes))return "null";
    return "{\"provider\":\"legacy_micro_video\",\"version\":"+std::to_string(m.version())+",\"flag\":"+std::to_string(m.legacy_micro_video())+",\"offset\":"+Text(std::to_string(m.micro_video_offset()))+",\"timeUs\":"+Text(std::to_string(m.primary_image_timestamp_us()))+"}";
  }
  if(type.find("OplusOliveMetadata")!=std::string::npos) {
    OplusOliveMetadata m;if(!m.ParseFromString(bytes))return "null";
    return "{\"provider\":\"oplus_olive\",\"version\":"+std::to_string(m.olive_photo_version())+",\"hasJxrs\":"+(m.has_jxrs_footer()?"true":"false")+",\"footerField\":"+std::to_string(m.jxrs_footer_version())+",\"checksum\":"+Text(m.jxrs_checksum_hex())+",\"videoLength\":"+Text(std::to_string(m.video_length()))+",\"timeUs\":"+Text(std::to_string(m.primary_image_timestamp_us()))+"}";
  }
  if(type.find("SamsungSefMetadata")!=std::string::npos) {
    SamsungSefMetadata m;if(!m.ParseFromString(bytes))return "null";std::string keys="[";
    for(int i=0;i<m.sef_keys_size();i++){if(i)keys+=',';keys+=Text(m.sef_keys(i));}keys+=']';
    return "{\"provider\":\"samsung_sef\",\"version\":"+std::to_string(m.sef_version())+",\"entries\":"+std::to_string(m.sef_entry_count())+",\"hasMotionData\":"+(m.has_motion_photo_data()?"true":"false")+",\"magic\":"+Text(m.motion_photo_data_magic())+",\"videoLength\":"+Text(std::to_string(m.video_length()))+",\"timeUs\":"+Text(std::to_string(m.primary_image_timestamp_us()))+",\"keys\":"+keys+"}";
  }
  if(type.find("HonorLivePhotoMetadata")!=std::string::npos) {
    HonorLivePhotoMetadata m;if(!m.ParseFromString(bytes))return "null";std::string tags="[";
    for(int i=0;i<m.tlv_tags_size();i++){if(i)tags+=',';tags+=Text(m.tlv_tags(i));}tags+=']';
    return "{\"provider\":\"honor_live_photo\",\"variant\":"+Text(m.format_variant())+",\"sceneMode\":"+std::to_string(m.scene_mode())+",\"videoLength\":"+Text(std::to_string(m.video_length()))+",\"timeUs\":"+Text(std::to_string(m.primary_image_timestamp_us()))+",\"tags\":"+tags+"}";
  }
  return "null";
}
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
      ",\"videoLength\":"+(b.has_video_length()?Text(std::to_string(b.video_length())):"null")+",\"semantic\":"+Semantic(b)+"}";
  }
  std::cout<<"METADATA_REFERENCE_JSON\t{\"declaresMotion\":"<<(declaration?"true":"false")
    <<",\"extracted\":"<<(extracted?"true":"false")<<",\"blocks\":"<<blocks<<"]}\n";
}
