package com.acme.bookstore;

import java.util.List;

public class Book {
    private Long id;
    private String title;
    private String author;
    private double price;
    private boolean available;

    public Book(String title, String author, double price) {
        this.title = title;
        this.author = author;
        this.price = price;
        this.available = true;
    }

    public Long getId() {
        return id;
    }

    public String getTitle() {
        return title;
    }

    public String getAuthor() {
        return author;
    }

    public double getPrice() {
        return price;
    }

    public boolean isAvailable() {
        return available;
    }

    public void setAvailable(boolean available) {
        this.available = available;
    }

    public String describe() {
        if (price > 50.0) {
            return title + " by " + author + " (premium)";
        } else {
            return title + " by " + author;
        }
    }
}
