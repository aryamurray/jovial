package com.acme.bookstore;

import java.util.List;
import java.util.Map;

public class BookController {
    private final BookService bookService;

    public BookController(BookService bookService) {
        this.bookService = bookService;
    }

    public List<Book> getAllBooks() {
        return bookService.listBooks();
    }

    public List<Book> getAvailableBooks() {
        return bookService.listAvailable();
    }

    public Book createBook(String title, String author, double price) {
        return bookService.addBook(title, author, price);
    }

    public boolean checkoutBook(String title) {
        return bookService.checkout(title);
    }

    public Map<String, Integer> getStats() {
        return bookService.getStats();
    }
}
