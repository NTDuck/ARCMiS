Feature: Tool catalog
  Agents add tools by name and find tools by name.

  Scenario: Add a tool
    Given an empty catalog
    When I add the tool "echo"
    Then the catalog holds 1 tool
    And the tool "echo" is present

  Scenario: Lookup misses before addition
    Given an empty catalog
    Then the tool "echo" is absent
