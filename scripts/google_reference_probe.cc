// Local differential adapter for the pinned public facade. Not a library dependency.
#include "libmotionphoto_api.h"
#include <fcntl.h>
#include <unistd.h>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

using namespace libmotionphoto::api;
void Print(const MotionPhotoMetadata& m, bool ok) {
  std::cout << "{\"recognized\":" << (ok ? "true" : "false")
    << ",\"video_length\":" << m.video_length
    << ",\"time_us\":" << m.presentation_timestamp_us
    << ",\"still_mime\":" << std::quoted(m.primary_image_mime)
    << ",\"video_mime\":" << std::quoted(m.video_mime) << "}";
}
int main(int argc, char** argv) {
  if (argc != 3) return 2;
  std::string path = argv[1], out = argv[2];
  std::ifstream input(path, std::ios::binary);
  std::vector<uint8_t> data((std::istreambuf_iterator<char>(input)), {});
  if (!input || data.empty()) return 3;
  MotionPhotoMetadata file, memory, range;
  auto quiet = [](const std::string&) {};
  bool f = ParseMotionPhotoFromFile(path, &file, quiet);
  bool m = ParseMotionPhotoFromMemory(data.data(), data.size(), &memory, quiet);
  std::string wrapper = out + "/wrapped.bin";
  { std::ofstream wrapped(wrapper, std::ios::binary); wrapped << "prefix-prefix";
    wrapped.write(reinterpret_cast<const char*>(data.data()), data.size()); wrapped << "suffix"; }
  int fd = open(wrapper.c_str(), O_RDONLY);
  bool r = ParseMotionPhotoFromFd(fd, 13, data.size(), &range, quiet);
  close(fd);
  ExtractorParams params;
  params.primary_image_output_path = out + "/reference-still.bin";
  params.video_output_path = out + "/reference-video.bin";
  params.metadata_output_path = out + "/reference-xmp.bin";
  bool extracted = m && ExtractMotionPhotoFromMemory(data.data(), data.size(), params, quiet);
  ExtractorParams from_file;
  from_file.motion_photo_filepath = path;
  from_file.primary_image_output_path = out + "/reference-file-still.bin";
  from_file.video_output_path = out + "/reference-file-video.bin";
  bool file_extracted = f && ExtractMotionPhotoFromFile(from_file, quiet);
  std::string agtm;
  if (m) ExtractAgtmFromMemory(data.data(), data.size(), [&](const std::string& s) {
    if (!s.empty() && s.front() == '{') agtm = s;
  });
  int check = CheckMotionPhotoFromMemory(data.data(), data.size(), quiet);
  std::cout << "REFERENCE_JSON\t{\"file\":"; Print(file,f);
  std::cout << ",\"memory\":"; Print(memory,m);
  std::cout << ",\"range\":"; Print(range,r);
  std::cout << ",\"check_memory_status\":" << check
    << ",\"extracted\":" << (extracted ? "true" : "false")
    << ",\"file_extracted\":" << (file_extracted ? "true" : "false")
    << ",\"metadata_memory_output_exists\":" << (std::ifstream(params.metadata_output_path).good() ? "true" : "false")
    << ",\"agtm\":" << (agtm.empty() ? "null" : agtm) << "}\n";
}
