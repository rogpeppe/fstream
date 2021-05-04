This repository holds a little proof-of-concept experiment, taking some ideas from an old [Inferno command](http://www.vitanuova.com/inferno/man/1/fs.html) and seeing how they might turn out when implemented in Rust.

The basic idea is to use a channel to transfer a stream of directory and file data and implement transformation primitives in terms of that, somewhat reminiscent of a Unix pipeline.

Unlike a Unix pipe, the channel protocol allows for feedback, so a task at the end of the pipeline can affect the code that's walking the directory, causing it to skip reading a file, for example.

To make things interesting, instead of using the channel directly, the primitives use an API that uses Rust ownership to make it impossible to use the protocol incorrectly. I can't quite decide if it works out quite nicely or it's harder to understand and more complex than necessary. :)
