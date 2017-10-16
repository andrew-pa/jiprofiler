// Copyright 2017 (c) Robert Palmer. All rights reserved.

package com.andrew.instrumentation.agent;

public class Program {

    private static long getFib(int n) {
        long a = 1;
        long b = 0;
        long answer = 1;
        for (int i = 0; i < n; i++) {
            answer = a + b;
            b = a;
            a = answer;
        }
        return answer;
    }

    private static long getFibRecursive(int n) {
        if (n <= 1) {
            return 1;
        } else {
            return getFibRecursive (n - 1) + getFibRecursive(n - 2);
        }
    }

    public static void main(String[] args) {
        int arg = 19;
        System.out.println("Fib number " + (arg + 1) + " is " + getFib(arg));
        System.out.println("Now recursively we get " + getFibRecursive(arg));
    }
}