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
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;

class Injector implements ClassFileTransformer {
    private Instrumentation instrumentation;
    private Set<String> classPathBlackList;

    public Injector(Instrumentation inst) {
        instrumentation = inst;
        classPathBlackList = new HashSet<>(Arrays.asList(
                "com/andrew/instrumentation/agent",
                "java", "sun", "jdk", "intellij"));
    }

    @Override
    public byte[] transform(ClassLoader loader, String className, Class<?> classBeingRedefined, ProtectionDomain protectionDomain, byte[] classfileBuffer) throws IllegalClassFormatException {
        for(String cnb : classPathBlackList)
            if(className.contains(cnb)) {
                //System.out.println("Ignoring blacklisted class " + className);
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
        if(methodName != null && !methodName.equals("")) {
            System.out.println(methodName);
            try {
                method.insertBefore("com.andrew.instrumentation.agent.PerformanceLogWriter.onEnterMethod(\"" + methodName + "\");");
                method.insertAfter("com.andrew.instrumentation.agent.PerformanceLogWriter.onExitMethod(\"" + methodName + "\");");
            } catch (Exception e) {
                e.printStackTrace();
            }
        }
    }
}