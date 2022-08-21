# riew

A minimalistic image viewer, written in Rust.


## How to run

Run `riew` with files or directories to view as parameters.

```
riew ~/nice-images/ some-image.png
```

Use `-d` to browse a directory starting at the given file.
This can be used as default action when opening an image file.

```
riew -d some-image.png
```


## Controls

| Event                        | Action                                                      |
|------------------------------|-------------------------------------------------------------|
| Arrow keys (when zoomed in)  | Pan the image                                               |
| Left/Right (when zoomed out) | Next/previous file, adjust zoom; use Shift to step 5 files  |
| PageDown/PageUp              |                                                             |
| Left/Right Click             |                                                             |
| Space/Backspace              | Scroll forth/back as pages, preserve zoom                   |
| F5                           | Refresh file list                                           |
| Left Mouse drag              | Pan the image; use Alt/Shift for smaller/larger steps       |
| Ctrl + Mouse move            | Display pixel information                                   |
| Mouse Wheel Up/Down          | Zoom in/out                                                 |
| + / -                        |                                                             |
| f                            | Toggle fullscreen                                           |
| a                            | Adjust zoom to fit image                                    |
| z                            | Reset zoom to 100%                                          |
| r / R                        | Rotate clockwise / counter-clockwise                        |
| Escape / q                   | Quit                                                        |


## Dependencies

This project depends on SDL2, SDL\_image and SDL2\_ttf libraries.


## License

This project is licensed under the MIT License.

Embedded DejaVu fonts uses [this free license](https://dejavu-fonts.github.io/License.html).

