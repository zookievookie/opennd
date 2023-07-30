# OpenND AVF Converter Beta

Converts AVF files to PNG.

## Compiling

Install Rust onto your system and cd into the source folder.
Build the project:

```sh
cargo build -r
```

## Usage

The tool takes an input and output path. The -p flag specifies the process to perform, AVF (single file) or a batch folder operation.
```sh
opennd-avf -p avf -i SonnyJoon.avf -o /Users/sonnyjoon/Desktop
```

Be sure to create a special output folder for batch operations, or else your desktop folder might end up really full!
```sh
opennd-avf -p batch -i CDVideo -o /Users/sonnyjoon/Desktop/CDVideo-converted
```