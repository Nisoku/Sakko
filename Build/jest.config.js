module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  roots: ["<rootDir>/tests"],
  collectCoverageFrom: [
    'src/**/*.ts',
    '!src/**/*.d.ts',
  ],
  coverageDirectory: 'coverage',
  coverageThreshold: {
    global: {
      branches: 70,
      functions: 70,
      lines: 70,
      statements: 70
    }
  },
  transform: {
    "^.+\\.tsx?$": ["ts-jest", {
      tsconfig: {
        target: "ES2020",
        module: "CommonJS",
        lib: ["ES2020", "DOM", "DOM.Iterable"],
        strict: true,
        esModuleInterop: true,
        skipLibCheck: true,
        moduleResolution: "node",
        isolatedModules: true,
      },
    }],
  },
};
