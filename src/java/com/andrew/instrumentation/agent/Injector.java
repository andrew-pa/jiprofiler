// Copyright (c) 2017 Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

import javassist.ClassPool;
import javassist.CtBehavior;
import javassist.CtClass;

import java.io.ByteArrayInputStream;
import java.lang.instrument.ClassFileTransformer;
import java.lang.instrument.IllegalClassFormatException;
import java.lang.instrument.Instrumentation;
import java.security.ProtectionDomain;

class Injector implements ClassFileTransformer {
    private Instrumentation instrumentation;

    public Injector(Instrumentation inst) {
        instrumentation = inst;
    }

    @Override
    public byte[] transform(ClassLoader loader, String className, Class<?> classBeingRedefined, ProtectionDomain protectionDomain, byte[] classfileBuffer) throws IllegalClassFormatException {
        if(className.contains("com/andrew/instrumentation/agent")) {
            return null;
        }
        return transformClass(classfileBuffer);
    }

    private byte[] transformClass(byte[] bytes) {
        ClassPool classPool = ClassPool.getDefault();
        CtClass ctClass = null;
        byte[] rewrittenClass = null;
        try {
            ctClass = classPool.makeClass(new ByteArrayInputStream(bytes));
            CtBehavior[] methods = ctClass.getDeclaredBehaviors();
            for (CtBehavior method : methods) {
                if (!method.isEmpty()) {
                    transformMethod(method);
                }
            }
            rewrittenClass = ctClass.toBytecode();
        } catch (Exception e) {
            e.printStackTrace();
        } finally {
            if (ctClass != null) {
                ctClass.detach();
            }
        }
        return rewrittenClass;
    }

    private void transformMethod(CtBehavior method) {
        String methodName = method.getLongName();
        try {
            method.insertBefore("com.andrew.instrumentation.agent.PerformanceLogWriter.onEnterMethod(" + methodName + ");");
            method.insertAfter("com.andrew.instrumentation.agent.PerformanceLogWriter.onExitMethod(" + methodName + ");");
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}