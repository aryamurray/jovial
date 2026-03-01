package com.acme.bookstore;

import java.util.List;
import java.util.Map;
import java.util.HashMap;

public class BookService {
    private final BookRepository bookRepository;

    public BookService(BookRepository bookRepository) {
        this.bookRepository = bookRepository;
    }

    public Book addBook(String title, String author, double price) {
        Book book = new Book(title, author, price);
        return bookRepository.save(book);
    }

    public List<Book> listBooks() {
        return bookRepository.findAll();
    }

    public List<Book> listAvailable() {
        return bookRepository.findAvailable();
    }

    public Map<String, Integer> getStats() {
        Map<String, Integer> stats = new HashMap<>();
        int total = bookRepository.count();
        int available = bookRepository.findAvailable().size();
        stats.put("total", total);
        stats.put("available", available);
        stats.put("checked_out", total - available);
        return stats;
    }

    public boolean checkout(String title) {
        var maybeBook = bookRepository.findByTitle(title);
        if (maybeBook.isEmpty()) {
            return false;
        }
        Book book = maybeBook.get();
        if (!book.isAvailable()) {
            return false;
        }
        book.setAvailable(false);
        return true;
    }
}
