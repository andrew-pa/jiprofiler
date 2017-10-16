// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

import java.lang.instrument.Instrumentation;

public class ProfilerAgent {
    public static void premain(String agentArgs, Instrumentation inst) {
        inst.addTransformer(new Injector(inst), false);
    }
}