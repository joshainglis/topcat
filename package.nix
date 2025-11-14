{
  lib,
  rustPlatform,

  versionCheckHook,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "topcat";
  version = "0.3.0";

  src = ./.;
  cargoHash = "sha256-YPgxoTm2earVQSz7QzBpAHMqiMHZndZkWPO8uHBZYxw=";

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
