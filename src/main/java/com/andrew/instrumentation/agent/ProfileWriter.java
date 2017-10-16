// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

interface ProfileWriter {

    void onEnterMethod(String methodName);
    void onExitMethod(String methodName);
}