// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

class PerformanceLogWritter {
    private static ProfileWriter profileWriter = new TextProfileWriter();

    public PerformanceLogWritter() {
    }

    public static void onEnterMethod(String methodName) {
        profileWriter.onEnterMethod(methodName);
    }

    public static void onExitMethod(String methodName) {
        profileWriter.onExitMethod(methodName);
    }
}