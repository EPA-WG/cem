<?xml version="1.0" encoding="UTF-8"?>
<!--
  cem-to-ce.xsl — CEM semantic markup → light-DOM custom-element markup

  Maps data-cem-* attributes to cem-{role} custom elements compatible with
  @epa-wg/custom-element. Tier A covers the nine core CEM roles.

  Usage (requires xsltproc or Saxon against XHTML5 input):
    xsltproc cem-to-ce.xsl fixture.xhtml

  Role mapping:
    data-cem-screen  → cem-screen[cem-id]
    data-cem-form    → cem-form[cem-id]
    data-cem-action  → cem-action[variant]
    data-cem-card    → cem-card[cem-id]
    data-cem-badge   → cem-badge[variant]
    data-cem-list    → cem-list[cem-id]
    data-cem-row     → cem-row[cem-id]
    data-cem-thread  → cem-thread[cem-id]
    data-cem-message → cem-message[variant]

  Roles with a cem-id receive an identifier attribute; roles that carry
  a semantic variant (action, badge, message) receive a variant attribute.
  All non-CEM attributes on the source element pass through unchanged.
-->
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">

  <xsl:output method="html" encoding="UTF-8" indent="no"/>

  <!-- Identity transform: copy everything by default -->
  <xsl:template match="@*|node()">
    <xsl:copy>
      <xsl:apply-templates select="@*|node()"/>
    </xsl:copy>
  </xsl:template>

  <!-- Suppress data-cem-* attributes on elements that are being replaced -->
  <xsl:template match="@*[starts-with(name(), 'data-cem-')]"/>

  <!-- data-cem-screen → cem-screen[cem-id] -->
  <xsl:template match="*[@data-cem-screen]">
    <cem-screen cem-id="{@data-cem-screen}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-screen>
  </xsl:template>

  <!-- data-cem-form → cem-form[cem-id] -->
  <xsl:template match="*[@data-cem-form]">
    <cem-form cem-id="{@data-cem-form}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-form>
  </xsl:template>

  <!-- data-cem-action → cem-action[variant] -->
  <xsl:template match="*[@data-cem-action]">
    <cem-action variant="{@data-cem-action}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-action>
  </xsl:template>

  <!-- data-cem-card → cem-card[cem-id] -->
  <xsl:template match="*[@data-cem-card]">
    <cem-card cem-id="{@data-cem-card}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-card>
  </xsl:template>

  <!-- data-cem-badge → cem-badge[variant] -->
  <xsl:template match="*[@data-cem-badge]">
    <cem-badge variant="{@data-cem-badge}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-badge>
  </xsl:template>

  <!-- data-cem-list → cem-list[cem-id] -->
  <xsl:template match="*[@data-cem-list]">
    <cem-list cem-id="{@data-cem-list}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-list>
  </xsl:template>

  <!-- data-cem-row → cem-row[cem-id] -->
  <xsl:template match="*[@data-cem-row]">
    <cem-row cem-id="{@data-cem-row}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-row>
  </xsl:template>

  <!-- data-cem-thread → cem-thread[cem-id] -->
  <xsl:template match="*[@data-cem-thread]">
    <cem-thread cem-id="{@data-cem-thread}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-thread>
  </xsl:template>

  <!-- data-cem-message → cem-message[variant] -->
  <xsl:template match="*[@data-cem-message]">
    <cem-message variant="{@data-cem-message}">
      <xsl:apply-templates select="@*|node()"/>
    </cem-message>
  </xsl:template>

</xsl:stylesheet>
