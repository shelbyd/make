# mk

Just make files/directories.

When making files, I often try to `touch foo/bar.txt`, but `foo/` doesn't exist. So I have to `mkdir foo; touch foo/bar.txt`. That's annoying.

## Installation

```
cargo install make
```

## Usage

`mk` just does the right thing. `mk foo/bar.txt` will create the directory `foo/` and then `bar.txt` as a regular file.

`mk` will infer if it should create a file or directory based on if the path has an extension. So `foo/bar` will be a directory, but `foo/bar.ext` will be a file. You can force a file to be created with `-f`, or a directory with `-d`.

`mk` can also take input from stdin. So `curl example.com | mk examples/example.com.txt` will create the `examples/` directory, the `example.com.txt` file, and pipe the input to that new file.

`mk` will mark created files as executable if they have an extension that is usually executable (`sh`, `exe`, `bat`, `jar`, and more).

`mk` will error if the file already exists, unless you specify `-o` for `--overwrite`.

## Potential Features

- [ ] Create temporary files/directories with `-t`
- [ ] User configurable templates for specific extensions
