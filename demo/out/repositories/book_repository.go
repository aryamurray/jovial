package bookstore

type BookRepository struct {
	Books []Book
	NextId int64
}

func (b *BookRepository) Save(book Book) Book {
	b.Books.Add(book)
	return book
}

func (b *BookRepository) FindAll() []Book {
	return b.Books
}

func (b *BookRepository) FindByTitle(title string) *Book {
	for _, book := range b.Books {
		if book.GetTitle().Equals(title) {
			return Optional.Of(book)
		}
	}
	return Optional.Empty()
}

func (b *BookRepository) FindAvailable() []Book {
	result := []interface{}{}
	for _, book := range b.Books {
		if book.IsAvailable() {
			result.Add(book)
		}
	}
	return result
}

func (b *BookRepository) Count() int {
	return b.Books.Size()
}
