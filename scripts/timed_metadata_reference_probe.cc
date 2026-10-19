// Test-only adapter to the pinned lightweight reader used by MetadataEngine.
#include "motion_photo_mp4/mp4_box_reader.h"
#include <iostream>
#include <string>
#include <vector>
int main(int argc, char** argv) {
  if (argc != 3) return 2;
  libmotionphoto::motion_photo::Mp4BoxReader reader;
  if (!reader.ParseFromFile(argv[1])) return 3;
  auto sample = reader.GetSampleIdFromTimeUs(2, std::stoll(argv[2]));
  std::vector<uint8_t> bytes;
  if (!reader.ReadSample(2, sample, &bytes)) return 4;
  std::cout << "{\"index\":" << sample - 1 << ",\"bytes\":[";
  for (size_t i = 0; i < bytes.size(); ++i) {
    if (i) std::cout << ',';
    std::cout << static_cast<unsigned>(bytes[i]);
  }
  std::cout << "]}\n";
}
