package com.acme.complex;

import java.util.List;
import java.util.ArrayList;
import java.util.Map;
import java.util.HashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.CompletableFuture;
import java.util.stream.Collectors;

// Annotation definitions for testing
@interface Cacheable {
    String value() default "";
    int ttl() default 300;
}

@interface Retry {
    int maxAttempts() default 3;
    long delay() default 1000;
}

@interface Validated {}

// Enum with methods
enum Priority {
    LOW(1), MEDIUM(5), HIGH(10), CRITICAL(100);

    private final int weight;

    Priority(int weight) {
        this.weight = weight;
    }

    public int getWeight() {
        return weight;
    }

    public boolean isUrgent() {
        return this == HIGH || this == CRITICAL;
    }
}

// Generic interface
interface Processor<T, R> {
    R process(T input) throws Exception;
    default boolean canProcess(T input) {
        return input != null;
    }
}

// Exception hierarchy
class TaskException extends RuntimeException {
    private final String taskId;

    public TaskException(String taskId, String message) {
        super(message);
        this.taskId = taskId;
    }

    public String getTaskId() {
        return taskId;
    }
}

class TaskTimeoutException extends TaskException {
    private final long timeoutMs;

    public TaskTimeoutException(String taskId, long timeoutMs) {
        super(taskId, "Task timed out after " + timeoutMs + "ms");
        this.timeoutMs = timeoutMs;
    }

    public long getTimeoutMs() {
        return timeoutMs;
    }
}

// Main class with annotations, generics, threading, lambdas, try-catch
@Validated
public class TaskProcessor implements Processor<String, Map<String, Object>> {

    private final ExecutorService executor;
    private final Map<String, Processor<String, ?>> handlers;
    private final List<String> completedTasks;
    private volatile boolean shutdown;
    private static final int MAX_POOL_SIZE = 10;
    private static int instanceCount = 0;

    // Static initializer block
    static {
        System.out.println("TaskProcessor class loaded");
    }

    public TaskProcessor() {
        this.executor = Executors.newFixedThreadPool(MAX_POOL_SIZE);
        this.handlers = new HashMap<>();
        this.completedTasks = new ArrayList<>();
        this.shutdown = false;
        instanceCount++;
    }

    // Method with multiple annotations
    @Cacheable(value = "results", ttl = 600)
    @Retry(maxAttempts = 5, delay = 2000)
    @Override
    public Map<String, Object> process(String taskId) throws TaskException {
        if (shutdown) {
            throw new TaskException(taskId, "Processor is shut down");
        }

        Map<String, Object> result = new HashMap<>();
        result.put("taskId", taskId);
        result.put("startTime", System.currentTimeMillis());

        // Try-catch with multiple catch blocks and finally
        try {
            Object output = executeTask(taskId);
            result.put("output", output);
            result.put("status", "SUCCESS");
        } catch (TaskTimeoutException e) {
            result.put("status", "TIMEOUT");
            result.put("timeoutMs", e.getTimeoutMs());
        } catch (TaskException e) {
            result.put("status", "FAILED");
            result.put("error", e.getMessage());
            throw e;
        } catch (Exception e) {
            result.put("status", "ERROR");
            result.put("error", e.getMessage());
            throw new TaskException(taskId, "Unexpected: " + e.getMessage());
        } finally {
            result.put("endTime", System.currentTimeMillis());
            synchronized (completedTasks) {
                completedTasks.add(taskId);
            }
        }

        return result;
    }

    // Async method using CompletableFuture and lambdas
    public CompletableFuture<Map<String, Object>> processAsync(String taskId) {
        return CompletableFuture.supplyAsync(() -> {
            try {
                return process(taskId);
            } catch (TaskException e) {
                throw new RuntimeException(e);
            }
        }, executor);
    }

    // Method using threads directly
    public void processBatch(List<String> taskIds) {
        List<Future<?>> futures = new ArrayList<>();

        for (String id : taskIds) {
            Future<?> future = executor.submit(() -> {
                try {
                    process(id);
                } catch (TaskException e) {
                    System.err.println("Task " + id + " failed: " + e.getMessage());
                }
            });
            futures.add(future);
        }

        // Wait for all to complete
        for (Future<?> future : futures) {
            try {
                future.get();
            } catch (Exception e) {
                // log and continue
                System.err.println("Future failed: " + e.getMessage());
            }
        }
    }

    // Generics with bounded wildcards
    public <T extends Comparable<T>> List<T> sortAndFilter(List<T> items, T threshold) {
        List<T> result = new ArrayList<>();
        for (T item : items) {
            if (item.compareTo(threshold) > 0) {
                result.add(item);
            }
        }
        // Simple sort using compareTo
        for (int i = 0; i < result.size() - 1; i++) {
            for (int j = i + 1; j < result.size(); j++) {
                if (result.get(i).compareTo(result.get(j)) > 0) {
                    T temp = result.get(i);
                    result.set(i, result.get(j));
                    result.set(j, temp);
                }
            }
        }
        return result;
    }

    // Ternary, casting, instanceof
    public String describeTask(Object task) {
        String description;
        if (task instanceof String) {
            description = "Simple task: " + (String) task;
        } else if (task instanceof Map) {
            Map<String, Object> map = (Map<String, Object>) task;
            description = "Complex task with " + map.size() + " properties";
        } else {
            description = task != null ? task.toString() : "null task";
        }
        return description;
    }

    // Static method
    public static int getInstanceCount() {
        return instanceCount;
    }

    // Inner class
    public class TaskResult {
        private final String taskId;
        private final Object output;
        private final long durationMs;

        public TaskResult(String taskId, Object output, long durationMs) {
            this.taskId = taskId;
            this.output = output;
            this.durationMs = durationMs;
        }

        public boolean isSlow() {
            return durationMs > 5000;
        }

        public String getSummary() {
            return taskId + ": " + (isSlow() ? "SLOW" : "OK") + " (" + durationMs + "ms)";
        }
    }

    private Object executeTask(String taskId) throws TaskException {
        // Simulate work
        try {
            Thread.sleep(100);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new TaskException(taskId, "Interrupted");
        }
        return "result-" + taskId;
    }

    public void shutdown() {
        this.shutdown = true;
        executor.shutdown();
    }
}
