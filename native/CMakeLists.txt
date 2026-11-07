# Builds libtorrent (vendored in ../third_party/libtorrent) together with the
# small C shim gt.cpp, so both share the same compile definitions.
cmake_minimum_required(VERSION 3.20)
project(grabbit_native CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(build_tests OFF CACHE BOOL "" FORCE)
set(build_examples OFF CACHE BOOL "" FORCE)
set(build_tools OFF CACHE BOOL "" FORCE)
set(python-bindings OFF CACHE BOOL "" FORCE)
set(deprecated-functions OFF CACHE BOOL "" FORCE)
set(encryption ON CACHE BOOL "" FORCE)

# A plain Boost source tree (headers only) has no CMake config files: use the
# classic module and point it at the headers. Otherwise rely on the system one.
if(DEFINED ENV{GRABBIT_BOOST_ROOT} AND NOT "$ENV{GRABBIT_BOOST_ROOT}" STREQUAL "")
  set(Boost_INCLUDE_DIR "$ENV{GRABBIT_BOOST_ROOT}" CACHE PATH "" FORCE)
  set(Boost_NO_BOOST_CMAKE ON)
  if(POLICY CMP0167)
    cmake_policy(SET CMP0167 OLD)
  endif()
endif()

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/libtorrent libtorrent EXCLUDE_FROM_ALL)

add_library(grabbit_gt STATIC gt.cpp)
target_link_libraries(grabbit_gt PRIVATE torrent-rasterbar)
