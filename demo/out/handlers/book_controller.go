package bookstore

type BookController struct {
	BookService BookService
}

func NewBookController(bookService BookService) *BookController {
	b.BookService = b.BookService
	return &BookController{}
}

func (b *BookController) GetAllBooks() []Book {
	return b.BookService.ListBooks()
}

func (b *BookController) GetAvailableBooks() []Book {
	return b.BookService.ListAvailable()
}

func (b *BookController) CreateBook(title string, author string, price float64) Book {
	return b.BookService.AddBook(title, author, price)
}

func (b *BookController) CheckoutBook(title string) bool {
	return b.BookService.Checkout(title)
}

func (b *BookController) GetStats() map[string]int {
	return b.BookService.GetStats()
}
