// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

import java.io.BufferedOutputStream;
import java.io.BufferedWriter;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.nio.Buffer;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.Stack;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipOutputStream;


class TextProfileWriter implements ProfileWriter {

    private ZipOutputStream zipOut;
    private BufferedOutputStream perfWriter;
    private ConcurrentHashMap<Long, Stack<Long>> startStack;
    private ConcurrentHashMap<String, Integer> methodMap;
    private Set<Long> threadIds;
    private long duration;
    private AtomicInteger methodIndex;
    private long absStartTime;

    public TextProfileWriter(String perfFilePath) {
        try {
            System.out.println("Writing performance data to: " + perfFilePath);
            zipOut = new ZipOutputStream(new FileOutputStream(perfFilePath));
            zipOut.putNextEntry(new ZipEntry("data"));
            perfWriter = new BufferedOutputStream(zipOut);
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
                    perfWriter.flush();
                    zipOut.closeEntry();
                    zipOut.putNextEntry(new ZipEntry("methods"));
                    for(Map.Entry<String, Integer> entry : methodMap.entrySet()) {
                        perfWriter.write((entry.getValue() + "|" + entry.getKey() + "\n").getBytes());
                    }

                    perfWriter.flush();
                    zipOut.closeEntry();

                    zipOut.putNextEntry(new ZipEntry("header"));
                    for(long t : threadIds) {
                        perfWriter.write((t+";").getBytes());
                    }
                    perfWriter.write(("\n"+duration).getBytes());

                    perfWriter.flush();
                    zipOut.closeEntry();
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
        Stack<Long> currentStartTimes =
                startStack.computeIfAbsent(currentThread.getId(), (Long l) -> new Stack<>());
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
            String line = (tid + "|" + (startTime-absStartTime) + "|" + (currentTime - startTime) +
                    "|" + methodId + "|" + callDepth + "\n");
            perfWriter.write(line.getBytes());
            duration = currentTime-absStartTime;
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}