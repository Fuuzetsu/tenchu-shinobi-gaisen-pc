{ pkgs }:
# winit/wgpu/cpal own presentation. Keep SDL's controller drivers without
# pulling another display/audio stack into the portable archives.
(pkgs.sdl3.override {
  alsaSupport = false;
  dbusSupport = false;
  drmSupport = false;
  ibusSupport = false;
  jackSupport = false;
  libdecorSupport = false;
  openglSupport = false;
  pipewireSupport = false;
  pulseaudioSupport = false;
  sndioSupport = false;
  traySupport = false;
  vulkanSupport = false;
  waylandSupport = false;
  x11Support = false;
}).overrideAttrs
  (old: {
    # These three suites initialize video unconditionally. Keep the remaining
    # upstream tests; the port also tests the gamepad adapter with virtual devices.
    checkPhase = ''
      runHook preCheck
      ctest --output-on-failure -j"$NIX_BUILD_CORES" -E '^(testautomation|testautomation-no-simd|testbounds)$'
      runHook postCheck
    '';
    cmakeFlags =
      old.cmakeFlags
      ++ [
        "-DSDL_AUDIO=OFF"
        "-DSDL_VIDEO=OFF"
        "-DSDL_GPU=OFF"
        "-DSDL_RENDER=OFF"
        "-DSDL_CAMERA=OFF"
      ]
      # SDL defaults DirectX to the video setting, but DirectInput needs it too.
      ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isWindows "-DSDL_DIRECTX=ON";
  })
