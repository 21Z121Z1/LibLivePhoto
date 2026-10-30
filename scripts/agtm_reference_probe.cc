// Independent adapter; compile only against the pinned Google reference source.
#include "motion_photo/agtm.h"
#include <fstream>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <vector>
int main(int argc, char** argv) {
  if (argc != 2) return 2;
  std::ifstream file(argv[1],std::ios::binary);
  std::vector<uint8_t> bytes((std::istreambuf_iterator<char>(file)),{});
  libmotionphoto::motion_photo::AgtmDynamicMetadata metadata;
  bool ok=libmotionphoto::motion_photo::ParseAgtmPayload(bytes.data(),bytes.size(),&metadata);
  std::cout << "{\"success\":" << (ok ? "true" : "false");
  if (ok) {
    std::cout << ",\"reference\":" << libmotionphoto::motion_photo::AgtmToJson(metadata)
      << ",\"chromaticities\":[";
    for (size_t i=0;i<8;++i) { if(i) std::cout << ',';std::cout << std::setprecision(9) << metadata.gain_application_space_chromaticities[i]; }
    std::cout << "],\"rules\":[";
    for (size_t i=0;i<metadata.rules.size();++i) {
      if(i) std::cout << ','; const auto& rule=metadata.rules[i];
      std::cout << "{\"alternateHdrHeadroom\":" << rule.alternate_hdr_headroom_log2
        << ",\"usePchipSlope\":" << (rule.use_pchip_slope ? "true":"false") << ",\"curve\":[";
      for(size_t j=0;j<rule.curve.size();++j) {
        if(j) std::cout << ',';const auto& p=rule.curve[j];
        std::cout << "{\"x\":" << p.x << ",\"y\":" << p.y << ",\"slope\":" << p.m << '}';
      }
      std::cout << "]}";
    }
    std::cout << ']';
  }
  std::cout << "}\n";
}
