package wire

import (
	"github.com/gin-gonic/gin"
	"github.com/acme/bookstore-go/handlers"
	"github.com/acme/bookstore-go/repositories"
	"github.com/acme/bookstore-go/services"
)

// App holds the initialized application components.
type App struct {
	Router *gin.Engine
}

// InitializeApp wires all dependencies and returns the application.
func InitializeApp() (*App, error) {
	// repositories
	bookRepository := NewBookRepository()

	// services
	bookService := NewBookService(bookRepository)

	// controllers
	bookController := NewBookController(bookService)

	router := gin.Default()

	router.GET("/api/books", bookController.GetAllBooks)
	router.GET("/api/books/available", bookController.GetAvailableBooks)
	router.POST("/api/books", bookController.CreateBook)
	router.POST("/api/books/checkout", bookController.CheckoutBook)
	router.GET("/api/books/stats", bookController.GetStats)

	return &App{Router: router}, nil
}
