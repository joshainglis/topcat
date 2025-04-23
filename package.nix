{
  lib,
  rustPlatform,

  versionCheckHook,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "topcat";
  version = "0.2.3";

  src = ./.;
  useFetchCargoVendor = true;
  cargoHash = "sha256-iVNAAzGwz1U17aP0bjxKj3ndPF1uXDtvuicp0g7HX+I=";

  nativeBuildInputs = [ ];

  buildInputs = [ ];

  nativeInstallCheckInputs = [
    versionCheckHook
  ];
  versionCheckProgramArg = [ "--version" ];
  doInstallCheck = true;

  meta = {
    description = "topological concatenation of files";
    homepage = "https://github.com/joshainglis/topcat";
    changelog = "https://github.com/joshainglis/topcat/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mit;
    mainProgram = "topcat";
    maintainers = with lib.maintainers; [
      joshainglis
    ];
  };
})
