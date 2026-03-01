package bookstore

type BookService struct {
	BookRepository BookRepository
}

func NewBookService(bookRepository BookRepository) *BookService {
	b.BookRepository = b.BookRepository
	return &BookService{}
}

func (b *BookService) AddBook(title string, author string, price float64) Book {
	book := NewBook(title, author, price)
	return b.BookRepository.Save(book)
}

func (b *BookService) ListBooks() []Book {
	return b.BookRepository.FindAll()
}

func (b *BookService) ListAvailable() []Book {
	return b.BookRepository.FindAvailable()
}

func (b *BookService) GetStats() map[string]int {
	stats := make(map[string]interface{})
	total := b.BookRepository.Count()
	available := b.BookRepository.FindAvailable().Size()
	stats.Put("total", total)
	stats.Put("available", available)
	stats.Put("checked_out", total - available)
	return stats
}

func (b *BookService) Checkout(title string) bool {
	maybeBook := b.BookRepository.FindByTitle(title)
	if maybeBook.IsEmpty() {
		return false
	}
	book := maybeBook.Get()
	if !book.IsAvailable() {
		return false
	}
	book.SetAvailable(false)
	return true
}
