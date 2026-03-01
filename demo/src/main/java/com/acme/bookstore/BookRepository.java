package com.acme.bookstore;

import java.util.List;
import java.util.ArrayList;
import java.util.Optional;

public class BookRepository {
    private final List<Book> books = new ArrayList<>();
    private Long nextId = 1L;

    public Book save(Book book) {
        books.add(book);
        return book;
    }

    public List<Book> findAll() {
        return books;
    }

    public Optional<Book> findByTitle(String title) {
        for (Book book : books) {
            if (book.getTitle().equals(title)) {
                return Optional.of(book);
            }
        }
        return Optional.empty();
    }

    public List<Book> findAvailable() {
        List<Book> result = new ArrayList<>();
        for (Book book : books) {
            if (book.isAvailable()) {
                result.add(book);
            }
        }
        return result;
    }

    public int count() {
        return books.size();
    }
}
