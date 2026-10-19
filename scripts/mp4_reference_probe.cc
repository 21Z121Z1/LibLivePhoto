// Test-only adapter to pinned Google code and its actual mp4v2 track-copy path.
// Does not expose or call Moments authoring.
#include "motion_photo_mp4/motion_photo_mp4_file_builder.h"
#include "image_io/base/message_handler.h"
#include <mp4v2/mp4v2.h>
#include <filesystem>
#include <iomanip>
#include <iostream>
#include <string>
using Builder = libmotionphoto::motion_photo::MotionPhotoMp4FileBuilder;
std::string JsonText(const std::string& text) {
  std::string out="\""; const char* hex="0123456789abcdef";
  for (unsigned char c:text) {
    if(c=='"'||c=='\\') {out+='\\';out+=c;}
    else if(c<32) {out+="\\u00";out+=hex[c>>4];out+=hex[c&15];}
    else out+=c;
  }
  return out+'"';
}
std::string Tracks(const std::string& path) {
  MP4FileHandle file=MP4Read(path.c_str());
  if(file==MP4_INVALID_FILE_HANDLE) return "null";
  std::string result="[";auto count=MP4GetNumberOfTracks(file,nullptr,0);
  if(count>4096){MP4Close(file);return "null";}
  for(uint32_t i=0;i<count;i++) {
    auto id=MP4FindTrackId(file,i,nullptr,0);const char* type=MP4GetTrackType(file,id);
    if(i)result+=',';
    result+="{\"id\":"+std::to_string(id)+",\"handler\":"+JsonText(type?type:"")+
      ",\"timescale\":"+std::to_string(MP4GetTrackTimeScale(file,id))+
      ",\"duration\":"+JsonText(std::to_string(MP4GetTrackDuration(file,id)))+
      ",\"samples\":"+std::to_string(MP4GetTrackNumberOfSamples(file,id))+"}";
  }
  MP4Close(file);return result+"]";
}
int main(int argc, char** argv) {
  if (argc < 3) return 2;
  photos_editing_formats::image_io::MessageHandler messages;
  messages.SetMessageWriter(nullptr);
  const std::string mode=argv[1], input=argv[2];
  const bool support=Builder::IsOptimizationSupported(input,&messages);
  bool ok=false; std::string summary;
  if (mode=="support") {
    ok=true; summary=Builder::GetFileTrackSummary(input,&messages);
  } else if (mode=="primary-optimize" && argc==4) {
    Builder builder(argv[3],&messages); size_t size=0;
    ok=builder.AddPrimaryMp4FileTracks(input,&summary) && support && builder.OptimizeMp4File(&summary,&size);
  } else if (mode=="strip" && argc==4) {
    std::filesystem::copy_file(input,argv[3]);
    ok=Builder::StripMetadataTrack(argv[3],&messages);
  } else if (mode=="copy-track" && argc==6) {
    // Exact primitive called by Google's internal import. Use a new copy.
    std::filesystem::copy_file(input,argv[3]);
    MP4FileHandle dst=MP4Modify(argv[3]), src=MP4Read(argv[4]);
    if (dst!=MP4_INVALID_FILE_HANDLE && src!=MP4_INVALID_FILE_HANDLE) {
      auto id=static_cast<MP4TrackId>(std::stoul(argv[5]));
      ok=MP4CopyTrack(src,id,dst,false,MP4_INVALID_TRACK_ID)!=MP4_INVALID_TRACK_ID;
    }
    if (src!=MP4_INVALID_FILE_HANDLE) MP4Close(src);
    if (dst!=MP4_INVALID_FILE_HANDLE) MP4Close(dst);
  } else return 2;
  const std::string input_tracks=Tracks(input), output_tracks=argc>3?Tracks(argv[3]):"null";
  std::cout<<"MP4_REFERENCE_JSON\t{\"success\":"<<(ok?"true":"false")
           <<",\"optimizationSupported\":"<<(support?"true":"false")
           <<",\"summary\":"<<JsonText(summary)<<",\"inputTracks\":"<<input_tracks
           <<",\"outputTracks\":"<<output_tracks<<"}\n";
}
