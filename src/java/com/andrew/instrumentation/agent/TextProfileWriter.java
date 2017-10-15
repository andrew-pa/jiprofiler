// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

import java.io.BufferedWriter;
import java.io.FileWriter;
import java.nio.file.Paths;
import java.util.Map;
import java.util.Stack;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Write one line per entry
 */
class TextProfileWriter implements ProfileWriter {

    private static final String perfFileSchema = "Thread|Start|Elapsed|Method|Depth\n";
    private static final String methodFileSchema = "Method|Name\n";
    private final String perfFilePrefix = "perfdata";
    private final String fileExtension = ".csv";
    private final String methodFilePrefix = "methodData";

    private BufferedWriter perfWriter;
    private ConcurrentHashMap<Long, Stack<Long>> startStack;
    private ConcurrentHashMap<String, Integer> methodMap;
    private AtomicInteger methodIndex;

    public TextProfileWriter() {
        try {
            String perfFilePath = Paths.get(perfFilePrefix + fileExtension).toAbsolutePath().toString();
            System.out.println("Writing performance data to: " + perfFilePath);
            perfWriter = new BufferedWriter(new FileWriter(perfFilePath));
            perfWriter.write(perfFileSchema);
            startStack = new ConcurrentHashMap<>();
            methodMap = new ConcurrentHashMap<>();
            methodIndex = new AtomicInteger(0);

        } catch (Exception e) {
            e.printStackTrace();
        }

        Runtime.getRuntime().addShutdownHook(new Thread() {
            public void run() {
                try {
                    perfWriter.flush();
                    perfWriter.close();

                    String methodFilePath = Paths.get(methodFilePrefix + fileExtension).toAbsolutePath().toString();
                    System.out.println("Writing method file to: " + methodFilePath);
                    BufferedWriter methodWriter = new BufferedWriter(new FileWriter(methodFilePath));
                    methodWriter.write(methodFileSchema);
                    for(Map.Entry<String, Integer> entry : methodMap.entrySet()) {
                        methodWriter.write(entry.getValue() + "|" + entry.getKey());
                    }
                    methodWriter.flush();
                    methodWriter.close();

                } catch (Exception e) {
                    e.printStackTrace();
                }
            }
        });
    }

    @Override
    public void onEnterMethod(String methodName) {
        assert methodName != null;
        Thread currentThread = Thread.currentThread();
        long startTime = System.nanoTime();
        Stack<Long> currentStartTimes = startStack.computeIfAbsent(currentThread.getId(), (Long l) -> new Stack<>());
        currentStartTimes.push(startTime);
        methodMap.computeIfAbsent(methodName, (String name) -> methodIndex.getAndIncrement());
    }

    @Override
    public void onExitMethod(String methodName) {
        assert methodName != null;

        Thread currentThread = Thread.currentThread();
        long tid = currentThread.getId();
        Stack<Long> threadStartTime = startStack.get(tid);
        int callDepth = threadStartTime.size();
        long startTime = threadStartTime.pop();
        long currentTime = System.nanoTime();
        int methodId = methodMap.get(methodName);
        try {
            perfWriter.write(tid + "|" + startTime + "|" + (currentTime - startTime) + "|" + methodId + "|" + callDepth);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}