// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

import java.io.BufferedWriter;
import java.io.FileWriter;
import java.nio.file.Paths;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.Stack;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Write one line per entry
 */
class TextProfileWriter implements ProfileWriter {

    private BufferedWriter perfWriter;
    private ConcurrentHashMap<Long, Stack<Long>> startStack;
    private ConcurrentHashMap<String, Integer> methodMap;
    private Set<Long> threadIds;
    private long duration;
    private AtomicInteger methodIndex;
    private long absStartTime;

    public TextProfileWriter(String perfFilePath) {
        try {
            System.out.println("Writing performance data to: " + perfFilePath);
            perfWriter = new BufferedWriter(new FileWriter(perfFilePath));
            startStack = new ConcurrentHashMap<>();
            methodMap = new ConcurrentHashMap<>();
            methodIndex = new AtomicInteger(0);
            threadIds = new HashSet<>();
            absStartTime = duration = 0L;
        } catch (Exception e) {
            e.printStackTrace();
        }

        Runtime.getRuntime().addShutdownHook(new Thread() {
            public void run() {
                try {

                    perfWriter.write("methods:\n");
                    for(Map.Entry<String, Integer> entry : methodMap.entrySet()) {
                        perfWriter.write(entry.getValue() + "|" + entry.getKey() + "\n");
                    }

                    perfWriter.write("threads:");
                    for(long t : threadIds) {
                        perfWriter.write(t+";");
                    }
                    perfWriter.write("\n");
                    perfWriter.write(""+duration);

                    perfWriter.flush();
                    perfWriter.close();
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
        if(absStartTime == 0) absStartTime = startTime;
        Stack<Long> currentStartTimes = startStack.computeIfAbsent(currentThread.getId(), (Long l) -> new Stack<>());
        currentStartTimes.push(startTime);
        methodMap.computeIfAbsent(methodName, (String name) -> methodIndex.getAndIncrement());
    }

    @Override
    public void onExitMethod(String methodName) {
        assert methodName != null;

        Thread currentThread = Thread.currentThread();
        long tid = currentThread.getId();
        threadIds.add(tid);
        Stack<Long> threadStartTime = startStack.get(tid);
        int callDepth = threadStartTime.size();
        long startTime = threadStartTime.pop();
        long currentTime = System.nanoTime();
        int methodId = methodMap.get(methodName);
        try {
            perfWriter.write(tid + "|" + (startTime-absStartTime) + "|" + (currentTime -
                    startTime) +
                    "|" + methodId + "|" + callDepth + "\n");
            duration = currentTime-absStartTime;
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}