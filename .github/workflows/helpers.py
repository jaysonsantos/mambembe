import hashlib
import os
import sys
from glob import glob
from subprocess import check_call
from tempfile import mkdtemp

TEMP = mkdtemp()

GHR_VERSION = "v0.13.0"
GHR_URL = f"https://github.com/tcnksm/ghr/releases/download/{GHR_VERSION}/{{file}}"
GHR_TRUSTED_CHECKSUMS = {
    "macos": "319988a001462f80b37cf40fbc41b9de60b0a1cffa2a338b47b9fe5eef25f60e",
    "ubuntu": "c428627270ae26e206cb526cb8c7bdfba475dd278f6691ddaf863355adadfa13",
    "windows": "e87485263f553ad64d4682967034a0a371ec9afa69ceecf4d1cd218ec4598381",
}


def get_running_os():
    return os.environ["OS"].split("-")[0]


def output_to_actions(name, value):
    """
    >>> output_to_actions("a-name", "a-value")
    ::set-output name=a-name::a-value
    """
    print(f"::set-output name={name}::{value}")


def output_tag():
    """
    >>> import os
    >>> os.environ.update(dict(GITHUB_REF="refs/tags/v1.2"))
    >>> output_tag()
    ::set-output name=TAG::v1.2
    """
    tag = get_tag_name()
    output_to_actions("TAG", tag)


def get_tag_name():
    """
    >>> import os
    >>> os.environ.update(dict(GITHUB_REF="refs/tags/v1.2"))
    >>> get_tag_name()
    'v1.2'
    """
    return os.environ["GITHUB_REF"].split("/")[-1]


def ghr():
    """
    >>> import os
    >>> os.environ.update(dict(OS="ubuntu-latest"))
    >>> ghr()  # doctest: +ELLIPSIS
    ::set-output name=GHR_BINARY_PATH::...ghr_v0.13.0_linux_amd64...ghr
    >>> os.environ["OS"] = "macos-latest"
    >>> ghr()  # doctest: +ELLIPSIS
    ::set-output name=GHR_BINARY_PATH::...ghr_v0.13.0_darwin_amd64...ghr
    >>> os.environ["OS"] = "windows-latest"
    >>> ghr()  # doctest: +ELLIPSIS
    ::set-output name=GHR_BINARY_PATH::...ghr_v0.13.0_windows_amd64...ghr.exe
    """
    running_os = get_running_os()

    os_map = {"ubuntu": "linux", "macos": "darwin"}
    ghr_os = os_map.get(running_os, running_os)
    ghr_extension = "tar.gz" if running_os == "ubuntu" else "zip"
    gh_binary_suffix = ".exe" if running_os == "windows" else ""

    ghr_folder = f"ghr_{GHR_VERSION}_{ghr_os}_amd64"
    ghr_file = f"{ghr_folder}.{ghr_extension}"
    ghr_file_destination = os.path.join(TEMP, ghr_file)

    os.makedirs("../", exist_ok=True)
    ghr_path = os.path.join(TEMP, ghr_folder, f"ghr{gh_binary_suffix}")

    download(GHR_URL.format(version=GHR_VERSION, file=ghr_file), ghr_file_destination)
    validate_checksum(ghr_file_destination, GHR_TRUSTED_CHECKSUMS[running_os])
    if running_os == "ubuntu":
        extract_command = ["tar", "xvf", ghr_file_destination, "-C", TEMP]
    else:
        extract_command = ["7z", "x", f"-o{TEMP}", ghr_file_destination]
    check_call(extract_command)

    output_to_actions("GHR_BINARY_PATH", ghr_path)


def download(url, file_name):
    check_call(["curl", "-sSfLo", file_name, url])


def validate_checksum(ghr_file, expected_hash):
    calculated = calculate_checksum(ghr_file)
    assert (
        calculated == expected_hash
    ), f"Failed to validate the hash of {ghr_file} expected {expected_hash} calculated {calculated}"


def calculate_checksum(filename):
    with open(filename, "rb") as f:
        final_hash = hashlib.sha256()
        while chunk := f.read(8192):
            final_hash.update(chunk)

        return final_hash.hexdigest()


def bundle():
    install_dir = os.environ["INSTALL_DIR"]
    target = os.environ["TARGET"]
    feature = os.environ["FEATURE"]
    collection_folder = "ghr_files"
    ghr_path = os.environ["GHR_BINARY_PATH"]
    tag_name = get_tag_name()

    for file_name in glob(os.path.join(install_dir, "**", "*")):
        split_file_name, suffix = os.path.splitext(os.path.basename(file_name))
        new_file_name = f"{split_file_name}-{feature}-{tag_name}-{target}{suffix}"
        new_file_destination = os.path.join(collection_folder, new_file_name)
        print(f"Copying file {file_name} to {new_file_destination}")
        os.renames(file_name, new_file_destination)
        compressed_destination = compress(new_file_destination)
        with open(f"{compressed_destination}.SHA256SUM", "wb") as f:
            checksum = calculate_checksum(compressed_destination)
            f.write(f"{checksum} {os.path.basename(compressed_destination)}\n".encode())

    check_call([ghr_path, tag_name, collection_folder])


def compress(file_name):
    directory = os.path.dirname(file_name)
    basename = os.path.basename(file_name)

    if get_running_os() == "ubuntu":
        output_file_name = f"{basename}.tar.gz"
        check_call(["tar", "czvf", output_file_name, basename], cwd=directory)
    else:
        output_file_name = f"{basename}.zip"
        check_call(["7z", "a", output_file_name, basename], cwd=directory)

    os.remove(file_name)

    return os.path.join(directory, output_file_name)


VALID_ACTIONS = {"output_tag": output_tag, "ghr": ghr, "bundle": bundle}


def main():
    args = sys.argv[1:]
    actions = [get_action(arg) for arg in args]
    if not actions:
        valid_actions = ", ".join(VALID_ACTIONS.keys())
        print(f"You must select at least one of {valid_actions}", file=sys.stderr)
    for action in actions:
        action()


def get_action(action):
    """
    >>> import os
    >>> os.environ.update(dict(GITHUB_REF="refs/tags/v1.2"))
    >>> get_action("unknown")()
    Traceback (most recent call last):
        ...
    KeyError: "Unknown action 'unknown' valid values are ['output_tag', 'ghr', 'bundle']"
    >>> get_action("output_tag")()
    ::set-output name=TAG::v1.2
    """
    try:
        return VALID_ACTIONS[action]
    except KeyError as exc:
        raise KeyError(
            f"Unknown action {action!r} valid values are {list(VALID_ACTIONS.keys())}"
        ) from exc


if __name__ == "__main__":
    main()
