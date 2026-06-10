# Java ConcurrentHashMap 가이드

## 개요
ConcurrentHashMap는 Java 표준 라이브러리의 핵심 클래스입니다.

## 주요 메서드
- `add()`, `remove()`, `get()`, `size()`

## 사용 예시
```java
ConcurrentHashMap obj = new ConcurrentHashMap<>();
obj.add("test");
```

## 스레드 안전성
동기화가 필요하면 `Collections.synchronized*()` 사용.

## 버전
Java 8부터 사용 가능.
