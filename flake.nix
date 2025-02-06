{
  inputs = {
    flakelight.url = "github:nix-community/flakelight";
    flakelight-rust.url = "github:accelbread/flakelight-rust";
  };
  outputs = { flakelight, flakelight-rust, ... }: flakelight ./. {
    imports = [ flakelight-rust.flakelightModules.default ];
    systems = [ "x86_64-linux" "aarch64-linux" "i686-linux" "armv7l-linux" "aarch64-darwin" ];
  };
}
