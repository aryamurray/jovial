package main

import (
	"github.com/acme/bookstore-go/internal/wire"
)

func main() {
	app, err := wire.InitializeApp()
	if err != nil {
		panic(err)
	}

	if err := app.Router.Run(":8080"); err != nil {
		panic(err)
	}
}
