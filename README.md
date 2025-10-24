# thl-patcher

A simple patching tool, which will evaluate the current version of your files, and compare it against an online source, to allow easy updates of your local files.

## Configuration

This program will read a `config.yaml` in the current directory, which will be configured as such:

### `config.yaml`
```yaml
window_name: "The name of your window"
source: "https://a-link-to-an-online/source.yaml"

# Paths in which the local files are expected to be
default_paths:
    # You can customize those paths based on the clients OS
  - target_os: windows
    possible_paths:
      - "C:\\Path\\To\\Files\\In\\Windows"
  - target_os: linux
    # When multiple paths are specified, the first path found is set as default
    possible_paths:
      - "~/path/to/files/in/linux"
      - "~/alternative/path/to/files/in/linux"
```

### `source.yaml`

Here is the specification of the `source.yaml`

```yaml
versions:
    # For each version, you specify a name. For now, names can be repeated, but this could change in a future version. 
  - name: "v0.0.0"
    # The `update_link` must point to the archive containing the patch allowing to update to the next version
    update_link: "http://localhost:8000/patch-v0.0.0-to-v0.1.0.tar.xz"
    # Determinants are a list of files that will be checked in order to determine which is the current version
    determinants:
      - file: file_1
        sha256: f09eb9f4aa1139cc9e04c8193f41adf2be4b31f3c779a85d217b2725732650e7
      - file: folder/file_2
        sha256: 12fd8f4ba62faf9ee53904333e90d46c30c620eaa3ccb1f17f72a50197ff7d05

  - name: "v0.1.0"
    determinants:
      - file: file_1
        sha256: 1fdf6aedec4911b1010734c457ef492c377076dd3376e48fe45c57becc2ed173
      - file: folder/file_2
        sha256: 2a83716c89fd1355acf02b11af563f3abe959e9d6cdb195ffaceffc796609198
```