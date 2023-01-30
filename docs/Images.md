Ironbar is capable of loading images from multiple sources. 
In any situation where an option takes text or an icon, 
you can use a string in any of the following formats, and it will automatically be detected as an image:

| Source                        | Example                         |
|-------------------------------|---------------------------------|
| GTK icon theme                | `icon:firefox`                  |
| Local file                    | `file:///path/to/file.jpg`      |
| Remote file (over HTTP/HTTPS) | `https://example.com/image.jpg` |

Remote images are loaded asynchronously to avoid blocking the UI thread. 
Be aware this can cause elements to change size upon load if the image is large enough.

Note that mixing text and images is not supported. 
Your best option here is to use Nerd Font icons instead.