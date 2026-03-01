package bookstore

type Book struct {
	Id int64
	Title string
	Author string
	Price float64
	Available bool
}

func NewBook(title string, author string, price float64) *Book {
	b.Title = b.Title
	b.Author = b.Author
	b.Price = b.Price
	b.Available = true
	return &Book{}
}

func (b *Book) GetId() int64 {
	return b.Id
}

func (b *Book) GetTitle() string {
	return b.Title
}

func (b *Book) GetAuthor() string {
	return b.Author
}

func (b *Book) GetPrice() float64 {
	return b.Price
}

func (b *Book) IsAvailable() bool {
	return b.Available
}

func (b *Book) SetAvailable(available bool) {
	b.Available = b.Available
}

func (b *Book) Describe() string {
	if b.Price > 50.0 {
		return b.Title + " by " + b.Author + " (premium)"
	} else {
		return b.Title + " by " + b.Author
	}
}
