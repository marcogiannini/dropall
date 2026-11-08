# Builds libtorrent (vendored in ../third_party/libtorrent) together with the
# small C shim gt.cpp, so both share the same compile definitions.
cmake_minimum_required(VERSION 3.20)
project(dropall_native CXX)

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

# A plain Boost source tree (headers only) has no CMake config files and the
# FindBoost module is gone in recent CMake: use the tiny package in boost-headers/.
# Without DROPALL_BOOST_ROOT the system Boost is used.
if(DEFINED ENV{DROPALL_BOOST_ROOT} AND NOT "$ENV{DROPALL_BOOST_ROOT}" STREQUAL "")
  set(Boost_DIR "${CMAKE_CURRENT_SOURCE_DIR}/boost-headers" CACHE PATH "" FORCE)
  set(CMAKE_FIND_PACKAGE_PREFER_CONFIG ON)
endif()

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../third_party/libtorrent libtorrent EXCLUDE_FROM_ALL)

add_library(dropall_gt STATIC gt.cpp)
target_link_libraries(dropall_gt PRIVATE torrent-rasterbar)
