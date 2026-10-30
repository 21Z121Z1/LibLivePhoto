// Read-only adapter for the pinned Google descriptor decoder, no Moments authoring.
#include "motion_photo/video_metadata_decoder.h"
#include <cstring>
#include <fstream>
#include <iostream>
#include <iterator>
#include <vector>
using namespace libmotionphoto::motion_photo;
void Frame(const FrameScoreDescriptor& f){uint32_t bits;float s=f.GetScore();std::memcpy(&bits,&s,4);std::cout<<"{\"scoreBits\":"<<bits<<",\"timeUs\":"<<f.GetPresentationTimestampUs()<<"}";}
int main(int argc,char**argv){if(argc!=2)return 2;std::ifstream file(argv[1],std::ios::binary);std::vector<uint8_t> bytes((std::istreambuf_iterator<char>(file)),{});VideoMetadataDecoder decoder;bool ok=decoder.DecodeMetadata(bytes);std::cout<<"{\"success\":"<<(ok?"true":"false");if(ok){const auto& m=decoder.GetMetadata();const auto&s=m.GetScoreDescriptor();std::cout<<",\"model\":"<<s.GetModelVersion()<<",\"primary\":";Frame(s.GetPrimaryImageFrameScoreDescriptor());std::cout<<",\"frames\":[";bool first=true;for(const auto& f:s.GetHighResTrackScoreDescriptor().GetFrameScoreDescriptors()){if(!first)std::cout<<",";first=false;Frame(f);}std::cout<<"],\"lowStabilized\":"<<(m.GetMetadataFlagsDescriptor().GetLowResTrackFlagsDescriptor().IsStabilized()?"true":"false")<<",\"highStabilized\":"<<(m.GetMetadataFlagsDescriptor().GetHighResTrackFlagsDescriptor().IsStabilized()?"true":"false");}std::cout<<"}\n";}
