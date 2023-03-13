# Parent project does not export its library target, so this CML implicitly
# depends on being added from it, i.e. the testing is done only from the build
# tree and is not feasible from an install location

project(cppTests LANGUAGES CXX)

# ---- Tests ----

add_executable(cpp_test source/cpp_test.cpp)
target_link_libraries(cpp_test PRIVATE cpp_lib)
target_compile_features(cpp_test PRIVATE cxx_std_17)

add_test(NAME cpp_test COMMAND cpp_test)

# ---- End-of-file commands ----

add_folders(Test)
