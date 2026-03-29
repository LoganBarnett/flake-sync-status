{ lib, ... }:
{
  options.flakeStatus = {
    online = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = lib.mdDoc ''
        Whether this host is reachable for flake-sync-status checks.
        When set to `false`, the CLI skips all connectivity checks and
        reports the host as offline immediately.  Set to `false` for
        hosts that are intentionally powered off or air-gapped.

        This option has no effect on the host itself; it is consumed
        only by the external flake-sync-status CLI tool.
      '';
    };
  };
}
