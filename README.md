# Chat Linker
The laziest discord bot ever made that links channels across servers

Simply cargo run it with a token env variable to run it
```console
$ token="never gonna give you up" cargo run
```

#### Flowchart of how to use it
 - You make a link with an ID
 - You link your channels with that link ID
 - **Boom, linked**

#### Commands
 - `/new (id) (title) (description)`

   Creates a link that other people can link their channels to

 - `/link (id) (#channel)`

   Links a channel to the specified link ID, and don't think you can execute this without having manage channels

 - `/list`

   Lists all available link IDs

This is just an interesting project, I don't have any plans to make this to a proper bot so you can do anything you want with it.
