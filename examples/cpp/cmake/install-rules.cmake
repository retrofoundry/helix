install(
    TARGETS cpp_exe
    RUNTIME COMPONENT cpp_Runtime
)

if(PROJECT_IS_TOP_LEVEL)
  include(CPack)
endif()
