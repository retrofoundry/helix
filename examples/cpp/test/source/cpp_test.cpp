#include "lib.hpp"

auto main() -> int
{
  auto const lib = library {};

  return lib.name == "cpp" ? 0 : 1;
}
